type PrimitiveTypes = string | number | boolean | Date | undefined | null;
type CollectionType =
  | object
  | Array<PrimitiveTypes | object>
  | Set<PrimitiveTypes>
  | Map<PrimitiveTypes, PrimitiveTypes | object>;

export type SupportedTypes = PrimitiveTypes | CollectionType;

export interface Serializable<T = any> {
  serialize(): Record<string, SupportedTypes>;
  /**
   * @description Use this to construct the object from a JSON object.
   * @param json - The JSON object to construct the object from.
   * @returns The constructed object.
   */
  deserialize(data: ReturnType<Serializable<T>['serialize']>): T;
}
