import { Given, When, Then, AfterAll, BeforeAll } from '@cucumber/cucumber';
import { expect } from 'chai';
import {
  decoratedModels,
  IdempotentRunnerResult,
  IdempotentTransformer,
  Serialize,
} from '@idempotent-transformer/core';
import { MessagePack } from '@idempotent-transformer/message-pack-adapter';
import { faker } from '@faker-js/faker';
import { IdempotentFactory } from '@idempotent-transformer/core';
import { CheckSumGenerator } from '@idempotent-transformer/checksum-adapter';
import { GrpcAdapter } from '@idempotent-transformer/grpc-adapter';

@Serialize({
  name: 'UserDefinedClass',
  serializeMethodName: 'toJSON',
  deserializeMethodName: 'fromJSON',
})
class UserDefinedClass {
  #a: string;
  #b: number;
  #c: Map<string, string>;
  #d: Set<number>;
  #e: Date;

  constructor(a: string, b: number, c: Map<string, string>, d: Set<number>, e: Date) {
    this.#a = a;
    this.#b = b;
    this.#c = c;
    this.#d = d;
    this.#e = e;
  }

  toJSON() {
    return {
      a: this.#a,
      b: this.#b,
      c: this.#c,
      d: this.#d,
      e: this.#e,
    };
  }

  static fromJSON(data: any) {
    return new UserDefinedClass(data.a, data.b, data.c, data.d, new Date(data.e));
  }

  sumSet() {
    return Array.from(this.#d).reduce((acc, curr) => acc + curr, 0);
  }

  get a() {
    return this.#a;
  }
  get b() {
    return this.#b;
  }
  get c() {
    return this.#c;
  }
  get d() {
    return this.#d;
  }
  get e() {
    return this.#e;
  }
}

let runner: IdempotentRunnerResult;
let firstResult: UserDefinedClass;
let secondResult: UserDefinedClass;
let transformer: IdempotentTransformer;
const workflowId = faker.string.uuid();

BeforeAll(async () => {
  const messagePack = MessagePack.getInstance();
  messagePack.configure(decoratedModels);
  transformer = await IdempotentFactory.getInstance().build({
    rpcAdapter: new GrpcAdapter({
      host: 'localhost',
      port: 51000,
    }),
    serializer: messagePack,
    logger: null,
    checksumGenerator: new CheckSumGenerator(),
  });
});

const userDefinedClassTask = async () => {
  return new UserDefinedClass(
    faker.string.uuid(),
    faker.number.int(),
    new Map().set('a', 'b'),
    new Set([1, 2, 3]),
    new Date()
  );
};
// Scenario: Serialize a user-defined class as a task result
// Given a user-defined class with private attributes
// And a task that returns an instance of this class
// When the task result is serialized
// Then the result can be decoded back to an instance of the user-defined class

// Setup transformer with in-memory storage for testing
Given(
  'a user-defined class with private attributes and a task that returns an instance of this class',
  async function () {
    // Defined in the beforeAll block
    runner = await transformer.startWorkflow(workflowId, {
      contextName: 'user-defined-class',
      isNested: false,
    });
  }
);

When('the task is executed twice to showcase a retry operation', async function () {
  firstResult = await runner.execute('user-defined-class', userDefinedClassTask);
  runner = await transformer.startWorkflow(workflowId, {
    contextName: 'user-defined-class',
    isNested: false,
  });
  secondResult = await runner.execute('user-defined-class', userDefinedClassTask);
  await runner.complete();
});

Then(
  'its result in second execution is decoded back to an instance of the class without the need any method to be called by developer',
  async function () {
    expect(secondResult.sumSet()).to.equal(6);
    expect(firstResult.a).to.equal(secondResult.a);
    expect(firstResult.b).to.equal(secondResult.b);
    expect(firstResult.c.get('a')).to.equal('b');
    expect(firstResult.d.has(3)).to.equal(true);
    expect(firstResult.e).to.deep.equal(secondResult.e);
  }
);

AfterAll(async () => {});
