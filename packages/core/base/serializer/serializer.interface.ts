type PrimitiveTypes = string | number | boolean | Date | undefined | null;
type CollectionType =
  | object
  | Array<PrimitiveTypes | object>
  | Set<PrimitiveTypes>
  | Map<PrimitiveTypes, PrimitiveTypes | object>;

type PlainObject = {
  [key: string]: PrimitiveTypes | PlainObject | Array<PrimitiveTypes | PlainObject>;
};

export type SupportedTypes = PrimitiveTypes | CollectionType | PlainObject;

export interface Serializable<T extends SupportedTypes = SupportedTypes> {
  serialize(): T;
  /**
   * @description Use this to construct the object from a JSON object.
   * @param json - The JSON object to construct the object from.
   * @returns The constructed object.
   */
  deserialize(data: ReturnType<Serializable<T>['serialize']>): T;
}
