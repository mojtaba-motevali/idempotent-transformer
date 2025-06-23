'use client';

import { IdempotentTransformer } from '@idempotent-transformer/core';
import { useContext, useRef, useState } from 'react';
import styles from './page.module.css';
import { IdempotentProviderContext } from '@/lib/context-api';

const mutationApiCall = async (): Promise<number> => {
  return new Promise((resolve) => {
    setTimeout(() => resolve(Math.floor(Math.random() * 100)), 500);
  });
};

const mutationApiCall2 = async (): Promise<number> => {
  return new Promise((resolve) => {
    setTimeout(() => resolve(Math.floor(Math.random() * 100)), 500);
  });
};

const generateRandomId = () => {
  return Math.random().toString(36).substring(2, 15);
};

interface IdempotencyCall {
  functionName: string;
  input: any[];
  output: any;
  fromCache: boolean;
  timestamp: string;
}

export default function Home() {
  const pageRef = useRef<string | null>(null);
  const idempotentTransformer = useContext(IdempotentProviderContext);
  if (pageRef.current === null) {
    pageRef.current = generateRandomId();
  }

  const [count, setCount] = useState(0);
  const [count2, setCount2] = useState(0);
  const [history, setHistory] = useState<IdempotencyCall[]>([]);
  const [loading, setLoading] = useState(false);

  const handleClick = async () => {
    setLoading(true);

    const idempotent = await idempotentTransformer!.makeIdempotent(pageRef.current!, {
      mutationApiCall,
      mutationApiCall2,
    });

    const result = await idempotent.mutationApiCall();
    setCount(result as number);
    setHistory((prev) => [
      {
        functionName: 'mutationApiCall',
        input: [],
        output: result,
        fromCache: false,
        timestamp: new Date().toLocaleTimeString(),
      },
      ...prev,
    ]);

    const result2 = await idempotent.mutationApiCall2();

    setCount2(result2 as number);
    setHistory((prev) => [
      {
        functionName: 'mutationApiCall2',
        input: [],
        output: result2,
        fromCache: false,
        timestamp: new Date().toLocaleTimeString(),
      },
      ...prev,
    ]);

    setLoading(false);
  };

  return (
    <div className={styles.container}>
      <h1>Hello World</h1>
      <div className={styles.historySection}>
        <h2>Idempotency Call History</h2>
        {history.length === 0 && <p className={styles.empty}>No idempotent calls yet.</p>}
        <div className={styles.historyList}>
          {history.map((call, idx) => (
            <div key={idx} className={styles.historyCard}>
              <div className={styles.cardHeader}>
                <span className={styles.functionName}>{call.functionName}</span>
                <span className={call.fromCache ? styles.cacheBadge : styles.freshBadge}>
                  {call.fromCache ? 'From Cache' : 'Fresh'}
                </span>
              </div>
              <div className={styles.cardBody}>
                <div>
                  <strong>Input:</strong> {JSON.stringify(call.input)}
                </div>
                <div>
                  <strong>Output:</strong> {JSON.stringify(call.output)}
                </div>
                <div className={styles.timestamp}>{call.timestamp}</div>
              </div>
            </div>
          ))}
        </div>
      </div>
      <button className={styles.ctaButton} onClick={handleClick} disabled={loading}>
        {loading ? 'Processing...' : 'Click me'}
      </button>
      <p>Count: {count}</p>
      <p>Count2: {count2}</p>
    </div>
  );
}
