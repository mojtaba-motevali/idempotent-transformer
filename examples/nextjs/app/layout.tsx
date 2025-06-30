'use client';

import { Geist, Geist_Mono } from 'next/font/google';
import './globals.css';
import { IdempotentFactory, IdempotentTransformer } from '@idempotent-transformer/core';
import { JsonSerializer, Md5Crypto } from '@/lib/idempotent';
import { InMemoryStorage } from '@/lib/storage';
import { useEffect, useState } from 'react';
import { IdempotentProviderContext } from '@/lib/context-api';

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
  const [idempotent, setIdempotent] = useState<IdempotentTransformer | null>(null);
  useEffect(() => {
    (async () => {
      await IdempotentFactory.build({
        serializer: new JsonSerializer(),
        crypto: new Md5Crypto(),
        storage: new InMemoryStorage(),
      });
      setIdempotent(IdempotentTransformer.getInstance());
    })();
  }, []);
  return (
    <html lang="en">
      <body className={`${geistSans.variable} ${geistMono.variable}`}>
        <IdempotentProviderContext.Provider value={idempotent}>
          {children}
        </IdempotentProviderContext.Provider>
      </body>
    </html>
  );
}
