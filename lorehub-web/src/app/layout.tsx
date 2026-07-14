import type { Metadata } from "next";
import { Inter } from "next/font/google";
import { AuthGate } from "@/components/AuthGate";
import "./globals.css";

const inter = Inter({
  variable: "--font-inter",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: "LoreHub",
  description:
    "Browse Lore repositories, review binary diffs, and manage access.",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className={`${inter.variable} h-full antialiased`}>
      <body className="min-h-full bg-bg-base text-text-primary">
        <AuthGate>{children}</AuthGate>
      </body>
    </html>
  );
}
