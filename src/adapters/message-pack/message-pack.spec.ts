import { Serialize } from './decorators/serialize.decorator';
import { MessagePack } from './message-pack';

describe('MessagePack', () => {
  it('should serialize and deserialize data', async () => {
    const messagePack = MessagePack.getInstance();
    const data = { name: 'John', age: 30 };
    const serialized = await messagePack.serialize(data);
    const deserialized = await messagePack.deserialize(serialized);
    expect(deserialized).toEqual(data);
  });

  it('should serialize and deserialize data with nested objects', async () => {
    const messagePack = MessagePack.getInstance();
    const data = { name: 'John', age: 30, address: { street: '123 Main St', city: 'Anytown' } };
    const serialized = await messagePack.serialize(data);
    const deserialized = await messagePack.deserialize(serialized);
    expect(deserialized).toEqual(data);
  });

  it('should serialize and deserialize data with arrays', async () => {
    const messagePack = MessagePack.getInstance();
    const data = { name: 'John', age: 30, hobbies: ['reading', 'traveling'] };
    const serialized = await messagePack.serialize(data);
    const deserialized = await messagePack.deserialize(serialized);
    expect(deserialized).toEqual(data);
  });

  it('should serialize and deserialize data with dates', async () => {
    const messagePack = MessagePack.getInstance();
    const data = { name: 'John', age: 30, date: new Date() };
    const serialized = await messagePack.serialize(data);
    const deserialized = await messagePack.deserialize(serialized);
    expect(deserialized).toEqual(data);
  });

  it('should serialize and deserialize data with booleans', async () => {
    const messagePack = MessagePack.getInstance();
    const data = { name: 'John', age: 30, isStudent: true };
    const serialized = await messagePack.serialize(data);
    const deserialized = await messagePack.deserialize(serialized);
    expect(deserialized).toEqual(data);
  });

  it('should serialize and deserialize data with all types of data', async () => {
    @Serialize({ name: 'Test' })
    class Test {
      #a: string;
      constructor(a: string) {
        this.#a = a;
      }

      toJSON() {
        return {
          a: this.#a,
        };
      }
      static fromJSON(json: any) {
        return new Test(json.a);
      }
    }

    @Serialize({ name: 'Test2' })
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
        return new Test2(json.a, new Test(json.b.a));
      }
    }

    const messagePack = MessagePack.getInstance();
    const data = {
      name: 'John',
      age: 30,
      isStudent: true,
      undefinedValue: undefined,
      nullValue: null,
      test: new Test('a'),
      test2: new Test2('b', new Test('c')),
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
      test: { a: 'a' },
      test2: { a: 'b', b: { a: 'c' } },
    });
  });
});
