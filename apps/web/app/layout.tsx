import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import "./globals.css";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: "FunButton — the dictation button for developers",
  description:
    "Hold Fn. Talk. Release. On-device Whisper + local AI cleanup types clean text at your cursor. No API key, no account, no cloud. GPLv3, lifetime pricing.",
  metadataBase: new URL("https://funbutton.ai"),
  openGraph: {
    title: "FunButton — the dictation button for developers",
    description:
      "Push-to-talk dictation for terminals, editors, and coding agents. Fully on-device, no API key ever. GPLv3 + lifetime pricing.",
    url: "https://funbutton.ai",
    siteName: "FunButton",
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title: "FunButton — the dictation button for developers",
    description:
      "Hold Fn. Talk. Clean text at your cursor, all on-device. No API key ever.",
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html
      lang="en"
      className={`${geistSans.variable} ${geistMono.variable} h-full antialiased`}
    >
      <body className="min-h-full flex flex-col">{children}</body>
    </html>
  );
}
