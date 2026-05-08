#!/usr/bin/env bash
# FunButton — bare-pipeline test script.
#
# Verifies the Groq Whisper + Llama 3.3 cleanup chain works on your network
# and credentials, without needing the full app installed.
#
# Usage:
#   GROQ_API_KEY=gsk_... ./scripts/test_pipeline.sh
#   GROQ_API_KEY=gsk_... ./scripts/test_pipeline.sh "your dictation here"
#
# What it does:
# 1. Synthesizes a WAV with macOS `say` from your phrase (or a default).
# 2. Sends it to Groq Whisper Turbo for transcription.
# 3. Sends the transcript to Groq Llama 3.3 with the FunButton CODE-mode prompt.
# 4. Prints raw vs cleaned so you can see the cleanup delta.
#
# Requires: macOS (for `say`), curl, jq, ffmpeg (`brew install jq ffmpeg`).
set -euo pipefail

if [[ -z "${GROQ_API_KEY:-}" ]]; then
  echo "GROQ_API_KEY env var required. Get one at https://console.groq.com/keys" >&2
  exit 1
fi

DICTATION="${1:-open paren camelCase user name comma age close paren arrow user name plus age dot to string open paren close paren}"
TMPDIR_FB=$(mktemp -d -t funbutton)
trap 'rm -rf "$TMPDIR_FB"' EXIT

echo "→ synthesizing WAV from: \"$DICTATION\""
say -o "$TMPDIR_FB/in.aiff" "$DICTATION"
ffmpeg -hide_banner -loglevel error -y -i "$TMPDIR_FB/in.aiff" -ar 16000 -ac 1 -f wav "$TMPDIR_FB/in.wav"

echo "→ POST $TMPDIR_FB/in.wav to Groq Whisper Turbo"
RAW=$(curl -fsS -X POST https://api.groq.com/openai/v1/audio/transcriptions \
  -H "Authorization: Bearer $GROQ_API_KEY" \
  -F "model=whisper-large-v3-turbo" \
  -F "response_format=json" \
  -F "temperature=0" \
  -F "file=@$TMPDIR_FB/in.wav" | jq -r '.text')
echo
echo "RAW TRANSCRIPT:"
echo "  $RAW"

CODE_PROMPT='You are FunButton in CODE mode. The user is dictating into a code editor or terminal. Convert their spoken instructions into the literal code/text they intended.

SPOKEN-SYMBOL VOCABULARY (always replace verbatim):
- "open paren" -> ( ; "close paren" -> )
- "open brace" / "open curly" -> { ; "close brace" / "close curly" -> }
- "open bracket" / "open square" -> [ ; "close bracket" -> ]
- "arrow" -> -> ; "fat arrow" -> => ; "thin arrow" -> ->
- "equals" -> = ; "double equals" -> == ; "triple equals" -> ===
- "comma" -> , ; "semicolon" -> ; ; "colon" -> :
- "dot" / "period" -> . ; "asterisk" / "star" -> *
- "underscore" -> _ ; "slash" -> / ; "backslash" -> \\
- "bang" / "exclamation" -> ! ; "dollar" -> $

IDENTIFIER CASING:
- "camelCase X Y Z" -> xYZ
- "PascalCase X Y Z" -> XYZ
- "snake_case X Y Z" -> x_y_z
- "kebab-case X Y Z" -> x-y-z

RULES:
1. Output ONLY the code/text to insert. No code fences, no commentary.
2. Preserve any literal identifiers the user spells out.
3. Do NOT add prose explanation.'

echo
echo "→ POST transcript to Groq Llama 3.3 70B (code mode prompt)"
CLEANED=$(jq -n --arg sys "$CODE_PROMPT" --arg user "$RAW" '{
  model: "llama-3.3-70b-versatile",
  temperature: 0.2,
  messages: [{role:"system",content:$sys},{role:"user",content:$user}]
}' | curl -fsS -X POST https://api.groq.com/openai/v1/chat/completions \
  -H "Authorization: Bearer $GROQ_API_KEY" \
  -H "Content-Type: application/json" \
  -d @- | jq -r '.choices[0].message.content')

echo
echo "CLEANED (CODE MODE):"
echo "  $CLEANED"
echo
echo "✓ Pipeline OK. The full app does the same thing automatically when you hold Right Option."
