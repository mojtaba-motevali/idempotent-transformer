type PrimitiveTypes = string | number | boolean | Date;
type CollectionType =
  | Array<PrimitiveTypes | object>
  | Set<PrimitiveTypes>
  | Map<PrimitiveTypes, PrimitiveTypes | object>;

type SupportedTypes = PrimitiveTypes | CollectionType;

export interface Serializable {
  toJSON(): Record<string, SupportedTypes>;
}

export type GenericModel = new (...args: any[]) => Serializable;

export interface MessagePackOptions {
  models?: {
    name: string;
    model: GenericModel;
  }[];
}
