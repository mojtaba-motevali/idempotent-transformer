import { MessagePack } from './message-pack';

describe('MessagePack', () => {
  it('should serialize and deserialize data', async () => {
    const messagePack = new MessagePack();
    const data = { name: 'John', age: 30 };
    const serialized = await messagePack.serialize(data);
    const deserialized = await messagePack.deserialize(serialized);
    expect(deserialized).toEqual(data);
  });

  it('should serialize and deserialize data with nested objects', async () => {
    const messagePack = new MessagePack();
    const data = { name: 'John', age: 30, address: { street: '123 Main St', city: 'Anytown' } };
    const serialized = await messagePack.serialize(data);
    const deserialized = await messagePack.deserialize(serialized);
    expect(deserialized).toEqual(data);
  });

  it('should serialize and deserialize data with arrays', async () => {
    const messagePack = new MessagePack();
    const data = { name: 'John', age: 30, hobbies: ['reading', 'traveling'] };
    const serialized = await messagePack.serialize(data);
    const deserialized = await messagePack.deserialize(serialized);
    expect(deserialized).toEqual(data);
  });

  it('should serialize and deserialize data with dates', async () => {
    const messagePack = new MessagePack();
    const data = { name: 'John', age: 30, date: new Date() };
    const serialized = await messagePack.serialize(data);
    const deserialized = await messagePack.deserialize(serialized);
    expect(deserialized).toEqual(data);
  });

  it('should serialize and deserialize data with booleans', async () => {
    const messagePack = new MessagePack();
    const data = { name: 'John', age: 30, isStudent: true };
    const serialized = await messagePack.serialize(data);
    const deserialized = await messagePack.deserialize(serialized);
    expect(deserialized).toEqual(data);
  });

  it('should serialize and deserialize data with undefined values', async () => {
    const messagePack = new MessagePack();
    const data = {
      name: 'John',
      age: 30,
      isStudent: true,
      undefinedValue: undefined,
      nullValue: null,
    };
    const serialized = await messagePack.serialize(data);
    const deserialized = await messagePack.deserialize(serialized);
    expect(deserialized).toEqual({ name: 'John', age: 30, isStudent: true, nullValue: null });
  });
});
