// Utility type to capitalize the first letter of a string
type Capitalize<S extends string> = S extends `${infer T}${infer U}` ? `${Uppercase<T>}${U}` : S;
// Utility type to convert snake_case to camelCase
type SnakeToCamelCase<S extends string> = S extends `${infer T}_${infer U}`
  ? `${T}${Capitalize<SnakeToCamelCase<U>>}`
  : S;
// Generic type transformer to convert DB entity type to DTO type

export type EntityToDto<T> = {
  [P in keyof T as P extends string ? SnakeToCamelCase<P> : never]: T[P] extends object
    ? EntityToDto<T[P]>
    : T[P];
};

type CamelCaseToSnake<S extends string> = S extends `${infer T}${infer U}`
  ? `${T extends Capitalize<T> ? `_${Lowercase<T>}` : T}${CamelCaseToSnake<U>}`
  : S;

export type DtoToEntity<T> = {
  [P in keyof T as P extends string ? CamelCaseToSnake<P> : never]: T[P];
};
