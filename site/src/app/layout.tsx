import { RootProvider } from 'fumadocs-ui/provider/next';
import './global.css';
import { Inter } from 'next/font/google';
import type { Metadata } from 'next';

const inter = Inter({
  subsets: ['latin'],
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
    <html lang="en" className={inter.className} suppressHydrationWarning>
      <body className="flex flex-col min-h-screen">
        <RootProvider search={{ options: { type: 'static' } }}>{children}</RootProvider>
      </body>
    </html>
  );
}
