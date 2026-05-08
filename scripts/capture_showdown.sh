#!/usr/bin/env bash
# Capture real Whisper raw + Llama cleaned outputs for the showdown page.
# Outputs JSON to apps/web/app/showdown/data.json
set -euo pipefail

if [[ -z "${GROQ_API_KEY:-}" ]]; then
  echo "GROQ_API_KEY required" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/apps/web/app/showdown/data.json"
TMPDIR_FB=$(mktemp -d -t funbutton-showdown)
trap 'rm -rf "$TMPDIR_FB"' EXIT
mkdir -p "$(dirname "$OUT")"

AUTO_PROMPT='You are FunButton, a voice dictation cleanup engine. Take the user'\''s transcribed speech and rewrite it as clean prose. Rules: (1) Remove filler words (um, uh, like, you know, sort of). (2) Fix grammar, punctuation, capitalization. (3) Resolve mid-sentence rewordings — if the user changed their mind mid-sentence, use the final version. (4) Preserve the speaker'\''s voice and tone — do NOT make it more formal than they were. (5) Output ONLY the cleaned text. No preamble, no quotes, no explanations.'

EMAIL_PROMPT='You are FunButton in EMAIL mode. Rewrite the user'\''s dictation as a clean email body. Rules: (1) Proper paragraphs and punctuation. (2) Fix grammar without making it overly formal. (3) Drop filler words. (4) Honor explicit dictated structure (e.g. '\''new paragraph'\'', '\''bullet point'\''). (5) Output ONLY the email body. No subject line unless dictated. No greeting/sign-off unless dictated.'

SLACK_PROMPT='You are FunButton in SLACK mode. Rewrite the user'\''s dictation as a casual chat message. Rules: (1) Keep it conversational — contractions, lowercase first word ok. (2) Drop filler words. (3) Preserve emoji intent if dictated ('\''thumbs up'\'' → 👍, '\''fire'\'' → 🔥). (4) No greetings or sign-offs. (5) Output ONLY the message text.'

CODE_PROMPT='You are FunButton in CODE mode. The user is dictating into a code editor or terminal. Convert their spoken instructions into the literal code/text they intended.

SPOKEN-SYMBOL VOCABULARY: open paren -> ( ; close paren -> ) ; open brace / open curly -> { ; close brace -> } ; open bracket -> [ ; close bracket -> ] ; arrow -> -> ; fat arrow -> => ; equals -> = ; double equals -> == ; comma -> , ; semicolon -> ; ; colon -> : ; dot -> . ; pipe -> | ; ampersand -> & ; star / asterisk -> * ; plus -> + ; minus -> - ; underscore -> _ ; slash -> / ; bang -> ! ; dollar -> $ ; at sign -> @ ; quote -> " ; backtick -> ` .

IDENTIFIER CASING: "camelCase X Y Z" -> xYZ ; "PascalCase X Y Z" -> XYZ ; "snake_case X Y Z" -> x_y_z ; "SCREAMING_SNAKE X Y Z" -> X_Y_Z ; "kebab-case X Y Z" -> x-y-z ; "dotted X Y Z" -> x.y.z .

RULES: (1) Output ONLY the code/text to insert. No code fences, no commentary. (2) Preserve any literal identifiers the user spells out. (3) Do NOT add prose explanation. (4) If the user dictates a comment sentence, preserve it verbatim minus filler.'

run_scenario() {
  local label="$1"
  local mode="$2"
  local prompt="$3"
  local utterance="$4"

  echo "→ [$label / $mode] synth + transcribe" >&2
  say -o "$TMPDIR_FB/$label.aiff" "$utterance"
  ffmpeg -hide_banner -loglevel error -y -i "$TMPDIR_FB/$label.aiff" -ar 16000 -ac 1 -f wav "$TMPDIR_FB/$label.wav"

  local raw
  raw=$(curl -fsS -X POST https://api.groq.com/openai/v1/audio/transcriptions \
    -H "Authorization: Bearer $GROQ_API_KEY" \
    -F "model=whisper-large-v3-turbo" \
    -F "response_format=json" \
    -F "temperature=0" \
    -F "file=@$TMPDIR_FB/$label.wav" | jq -r '.text' | sed -E 's/^[[:space:]]+//;s/[[:space:]]+$//')

  echo "  RAW: $raw" >&2

  local cleaned
  cleaned=$(jq -n --arg sys "$prompt" --arg user "$raw" '{
    model: "llama-3.3-70b-versatile",
    temperature: 0.2,
    messages: [{role:"system",content:$sys},{role:"user",content:$user}]
  }' | curl -fsS -X POST https://api.groq.com/openai/v1/chat/completions \
    -H "Authorization: Bearer $GROQ_API_KEY" \
    -H "Content-Type: application/json" \
    -d @- | jq -r '.choices[0].message.content' | sed -E 's/^[[:space:]]+//;s/[[:space:]]+$//')

  echo "  CLEANED: $cleaned" >&2
  echo "" >&2

  jq -n --arg label "$label" --arg mode "$mode" --arg utterance "$utterance" --arg raw "$raw" --arg cleaned "$cleaned" \
    '{label:$label, mode:$mode, utterance:$utterance, raw:$raw, cleaned:$cleaned}'
}

declare -a OUTPUTS=()

S1=$(run_scenario "rambling-email" "email" "$EMAIL_PROMPT" "Hi um Russell I wanted to like you know follow up about the deal we discussed last week and uh I think we should probably move forward but I'm not sure about the pricing maybe we could do something like 30K instead of 25 since uh you know the scope grew a bit. Let me know what you think.")
OUTPUTS+=("$S1")

S2=$(run_scenario "slack-correction" "slack" "$SLACK_PROMPT" "Hey can you ship that PR by EOD wait actually no make it tomorrow morning since the build is broken in CI right now")
OUTPUTS+=("$S2")

S3=$(run_scenario "code-arrow-fn" "code" "$CODE_PROMPT" "open paren camelCase user data comma options close paren fat arrow object dot keys open paren user data close paren dot filter open paren camelCase k fat arrow not k dot starts with open paren quote underscore quote close paren close paren")
OUTPUTS+=("$S3")

S4=$(run_scenario "snake-case-fn" "code" "$CODE_PROMPT" "function snake_case fetch user profile takes user id of type number returns promise of user")
OUTPUTS+=("$S4")

S5=$(run_scenario "mid-sentence-redirect" "auto" "$AUTO_PROMPT" "Let me draft a quick response to that customer about um the refund situation actually no scratch that I'll handle it tomorrow morning when I have my notes in front of me")
OUTPUTS+=("$S5")

# Combine into a JSON array
printf '%s\n' "${OUTPUTS[@]}" | jq -s '.' > "$OUT"
echo "Wrote $OUT"
echo "Scenarios captured: $(jq 'length' "$OUT")"
