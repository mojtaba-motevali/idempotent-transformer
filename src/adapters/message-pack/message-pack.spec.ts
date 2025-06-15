import { Serialize } from '../../lib/base/serializer/serialize.decorator';
import { MessagePack } from './message-pack';

@Serialize({ name: 'Lolita' })
class Test {
  #a: string;
  #b: number;
  constructor(a: string, b: number) {
    this.#a = a;
    this.#b = b;
  }

  toJSON() {
    return {
      a: this.#a,
      b: this.#b,
    };
  }
  static fromJSON(json: any) {
    return new Test(json.a, json.b);
  }
  getB() {
    return this.#b + 20;
  }
}

@Serialize({ name: 'MyTest2' })
class Test2 {
  constructor(
    public a: string,
    public b: Test
  ) {}

  toJSON() {
    return {
      a: this.a,
      b: this.b.toJSON(),
    };
  }
  static fromJSON(json: ReturnType<Test2['toJSON']>) {
    return new Test2(json.a, new Test(json.b.a, json.b.b));
  }
}

describe('MessagePack', () => {
  let messagePack: MessagePack;
  beforeAll(() => {
    messagePack = MessagePack.getInstance();
    messagePack.configure();
  });

  it('should serialize and deserialize data', async () => {
    const data = { name: 'John', age: 30 };
    const serialized = await messagePack.serialize(data);
    const deserialized = await messagePack.deserialize(serialized);
    expect(deserialized).toEqual(data);
  });

  it('should serialize and deserialize data with nested objects', async () => {
    const data = { name: 'John', age: 30, address: { street: '123 Main St', city: 'Anytown' } };
    const serialized = await messagePack.serialize(data);
    const deserialized = await messagePack.deserialize(serialized);
    expect(deserialized).toEqual(data);
  });

  it('should serialize and deserialize data with arrays', async () => {
    const data = { name: 'John', age: 30, hobbies: ['reading', 'traveling'] };
    const serialized = await messagePack.serialize(data);
    const deserialized = await messagePack.deserialize(serialized);
    expect(deserialized).toEqual(data);
  });

  it('should serialize and deserialize data with dates', async () => {
    const data = { name: 'John', age: 30, date: new Date() };
    const serialized = await messagePack.serialize(data);
    const deserialized = await messagePack.deserialize(serialized);
    expect(deserialized).toEqual(data);
  });

  it('should serialize and deserialize data with booleans', async () => {
    const data = { name: 'John', age: 30, isStudent: true };
    const serialized = await messagePack.serialize(data);
    const deserialized = await messagePack.deserialize(serialized);
    expect(deserialized).toEqual(data);
  });

  it('should serialize and deserialize data with all types of data', async () => {
    const data = {
      name: 'John',
      age: 30,
      isStudent: true,
      undefinedValue: undefined,
      nullValue: null,
      test: new Test('a', 10),
      test2: new Test2('b', new Test('c', 20)),
    };
    const serialized = await messagePack.serialize(data);
    const deserialized: any = await messagePack.deserialize(serialized);
    expect({
      ...deserialized,
      test: deserialized.test.toJSON(),
      test2: deserialized.test2.toJSON(),
    }).toEqual({
      name: 'John',
      age: 30,
      isStudent: true,
      nullValue: null,
      test: { a: 'a', b: 10 },
      test2: { a: 'b', b: { a: 'c', b: 20 } },
    });
    expect(deserialized.test.getB()).toEqual(30);
  });
});
