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
  title: "FunButton.ai — coming soon",
  description:
    "Talk fast. Stay local. Pay less. Voice dictation for people who actually ship. Built in public.",
  metadataBase: new URL("https://funbutton.ai"),
  openGraph: {
    title: "FunButton.ai — coming soon",
    description:
      "Talk fast. Stay local. Pay less. Wispr Flow without the SaaS. Built in public.",
    url: "https://funbutton.ai",
    siteName: "FunButton",
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title: "FunButton.ai — coming soon",
    description: "Talk fast. Stay local. Pay less. Built in public.",
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
