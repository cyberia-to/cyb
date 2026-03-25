type KeyValue<T> = {
  [key: string]: T;
};

export type ObjKeyValue = KeyValue<number>;

export type KeyValueString = KeyValue<string>;

export type TabularKeyValues = { [key: string]: KeyValueString };

type ObjectKey<T> = {
  [key: string | number]: T;
};
