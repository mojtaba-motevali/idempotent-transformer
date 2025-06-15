'use client';

import { Geist, Geist_Mono } from 'next/font/google';
import './globals.css';
import { IdempotentFactory } from '@idempotent-transformer/core';
import { JsonSerializer, Md5Crypto, Storage } from '@/lib/idempotent';
import { useEffect } from 'react';

const geistSans = Geist({
  variable: '--font-geist-sans',
  subsets: ['latin'],
});

const geistMono = Geist_Mono({
  variable: '--font-geist-mono',
  subsets: ['latin'],
});

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  useEffect(() => {
    (async () => {
      await IdempotentFactory.build({
        serializer: new JsonSerializer(),
        crypto: new Md5Crypto(),
        storage: new Storage(),
        logger: {
          debug: (message) => {
            console.log(message);
          },
          error: (message) => {
            console.error(message);
          },
        },
      });
    })();
  }, []);
  return (
    <html lang="en">
      <body className={`${geistSans.variable} ${geistMono.variable}`}>{children}</body>
    </html>
  );
}
