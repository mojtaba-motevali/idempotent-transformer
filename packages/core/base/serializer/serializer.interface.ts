type PrimitiveTypes = string | number | boolean | Date;
type CollectionType =
  | object
  | Array<PrimitiveTypes | object>
  | Set<PrimitiveTypes>
  | Map<PrimitiveTypes, PrimitiveTypes | object>;

type SupportedTypes = PrimitiveTypes | CollectionType;

export interface Serializable<T = any> {
  toJSON(): Record<string, SupportedTypes>;
  /**
   * @description Use this to construct the object from a JSON object.
   * @param json - The JSON object to construct the object from.
   * @returns The constructed object.
   */
  fromJSON(json: ReturnType<Serializable<T>['toJSON']>): T;
}
