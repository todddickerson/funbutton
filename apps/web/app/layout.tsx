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
  title: "FunButton.ai — talk fast, stay local, pay less",
  description:
    "A dev-grade voice dictation tool. Press Right Option, talk, watch your cleaned-up text land at the cursor. Local-first. Code-aware. Half the price of Wispr Flow.",
  metadataBase: new URL("https://funbutton.ai"),
  openGraph: {
    title: "FunButton.ai — talk fast, stay local, pay less",
    description:
      "Dev-grade voice dictation. Local-first. Code-aware (camelCase, snake_case, spoken symbols). GPLv3 desktop core. Half the price of Wispr.",
    url: "https://funbutton.ai",
    siteName: "FunButton",
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title: "FunButton.ai",
    description: "Dev-grade voice dictation. Local-first. Code-aware. Half the price of Wispr.",
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
