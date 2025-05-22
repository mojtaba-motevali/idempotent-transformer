import { createClient } from 'redis';
import { randomUUID } from 'crypto';

const STREAM_KEY = 'raft:cluster:messages';
const GROUP_NAME = 'raft-group';
const NODE_ID = process.env.NODE_ID || randomUUID();
const ELECTION_TIMEOUT = 3000 + Math.floor(Math.random() * 2000);

let currentTerm = 0;
let votedFor: string | null = null;
let leaderId: string | null = null;

const redis = createClient();

async function initStreamGroup() {
  try {
    await redis.xGroupCreate(STREAM_KEY, GROUP_NAME, '0', { MKSTREAM: true });
  } catch (e: any) {
    if (!e.message.includes('BUSYGROUP')) throw e;
  }
}

async function sendMessage(message: any) {
  await redis.xAdd(STREAM_KEY, '*', { message: JSON.stringify(message) });
}

async function requestExecution(operationId: string) {
  currentTerm++;
  votedFor = NODE_ID;
  leaderId = null;

  console.log(
    `[${NODE_ID}] Requesting to execute operation '${operationId}' in term ${currentTerm}`
  );
  await sendMessage({
    type: 'RequestVote',
    from: NODE_ID,
    term: currentTerm,
    operationId,
  });

  return new Promise<void>((resolve) => {
    const timeout = setTimeout(() => {
      if (leaderId === NODE_ID) {
        resolve();
      }
    }, ELECTION_TIMEOUT + 1000);

    const checkLeader = setInterval(() => {
      if (leaderId === NODE_ID) {
        clearInterval(checkLeader);
        clearTimeout(timeout);
        resolve();
      }
    }, 100);
  });
}

async function completeExecution(operationId: string) {
  await sendMessage({
    type: 'ExecutionComplete',
    from: NODE_ID,
    term: currentTerm,
    operationId,
  });
}

async function handleMessage(message: any) {
  const { type, term, from, operationId } = message;

  if (term > currentTerm) {
    currentTerm = term;
    votedFor = null;
    leaderId = null;
  }

  switch (type) {
    case 'RequestVote':
      if (!votedFor || votedFor === from) {
        votedFor = from;
        await sendMessage({
          type: 'VoteResponse',
          from: NODE_ID,
          to: from,
          term: currentTerm,
          operationId,
        });
      }
      break;
    case 'VoteResponse':
      if (!leaderId && term === currentTerm) {
        console.log(`[${NODE_ID}] Received vote from ${from} for operation '${operationId}'`);
        leaderId = NODE_ID;
      }
      break;
    case 'ExecutionComplete':
      console.log(`[${NODE_ID}] Operation '${operationId}' completed by ${from}`);
      leaderId = null;
      break;
  }
}

async function pollMessages() {
  while (true) {
    const response = await redis.xReadGroup(
      GROUP_NAME,
      NODE_ID,
      { key: STREAM_KEY, id: '>' },
      { COUNT: 10, BLOCK: 5000 }
    );

    if (!Array.isArray(response)) continue;

    for (const stream of response) {
      const messages = (stream as { messages: { id: string; message: { message: string } }[] })
        .messages;
      if (!Array.isArray(messages)) continue;

      for (const msg of messages) {
        const id = msg.id;
        const fields = msg.message;
        const raw = fields.message;
        if (!raw) continue;

        try {
          const message = JSON.parse(raw);
          await handleMessage(message);
          await redis.xAck(STREAM_KEY, GROUP_NAME, id);
        } catch (err) {
          console.error(`[${NODE_ID}] Failed to parse or handle message:`, err);
        }
      }
    }
  }
}

export async function runWithElection(operationId: string, fn: () => Promise<void>) {
  await requestExecution(operationId);
  if (leaderId === NODE_ID) {
    console.log(`[${NODE_ID}] Executing operation '${operationId}'`);
    await fn();
    await completeExecution(operationId);
  } else {
    console.log(`[${NODE_ID}] Waiting for leader to complete operation '${operationId}'`);
  }
}

(async () => {
  await redis.connect();
  await initStreamGroup();
  await pollMessages();
})();
