import { RootProvider } from 'fumadocs-ui/provider/next';
import './global.css';
import { Geist, Geist_Mono } from 'next/font/google';
import type { Metadata } from 'next';

const geist = Geist({
  subsets: ['latin'],
  variable: '--font-geist-sans',
});

const geistMono = Geist_Mono({
  subsets: ['latin'],
  variable: '--font-geist-mono',
});

export const metadata: Metadata = {
  title: 'wf — workflow CLI',
  description: 'Parameterized shell commands. Interactive prompts. Copied to your clipboard.',
  icons: {
    icon: '/icon.svg',
  },
  openGraph: {
    title: 'wf-cli',
    description: 'Parameterized shell commands. Interactive prompts. Copied to your clipboard.',
    url: 'https://wf.sagoez.com',
    siteName: 'wf-cli',
    images: [{ url: 'https://wf.sagoez.com/og.png', width: 1200, height: 630 }],
    type: 'website',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'wf-cli',
    description: 'Parameterized shell commands. Interactive prompts. Copied to your clipboard.',
    images: ['https://wf.sagoez.com/og.png'],
  },
};

export default function Layout({ children }: LayoutProps<'/'>) {
  return (
    <html lang="en" className={`${geist.variable} ${geistMono.variable} ${geist.className}`} data-scroll-behavior="smooth" suppressHydrationWarning>
      <body className="flex flex-col min-h-screen">
        <RootProvider search={{ options: { type: 'static' } }}>{children}</RootProvider>
      </body>
    </html>
  );
}
