'use client';

import { IdempotentTransformer } from '@idempotent-transformer/core';
import { useRef, useState } from 'react';

const mutationApiCall = async (): Promise<number> => {
  return new Promise((resolve) => {
    resolve(Math.random() * 100);
  });
};

const mutationApiCall2 = async (): Promise<number> => {
  return new Promise((resolve) => {
    resolve(Math.random() * 100);
  });
};

export default function Home() {
  const pageRef = useRef<string | null>(null);

  if (pageRef.current === null) {
    pageRef.current = 'random-id';
  }

  const [count, setCount] = useState(0);
  const [count2, setCount2] = useState(0);

  const handleClick = async () => {
    const idempotent = await IdempotentTransformer.getInstance().makeIdempotent(pageRef.current!, {
      mutationApiCall: async (...args: Parameters<typeof mutationApiCall>) =>
        mutationApiCall(...args),
      mutationApiCall2: async (...args: Parameters<typeof mutationApiCall2>) =>
        mutationApiCall2(...args),
    });
    const result = await idempotent.mutationApiCall();
    setCount(result as number);

    const result2 = await idempotent.mutationApiCall2();
    setCount2(result2 as number);
  };

  return (
    <div>
      <h1>Hello World</h1>
      <button onClick={handleClick}>Click me</button>
      <p>Count: {count}</p>
      <p>Count2: {count2}</p>
    </div>
  );
}
