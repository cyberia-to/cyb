import { DbEntity } from './types/entities';
import { Column, DBSchema, GetCommandOptions } from './types/types';
import { entityToArray } from './utils';

export function createCozoDbCommandFactory(dbSchema: DBSchema) {
  const schema = dbSchema;

  const generateRmCommand = (tableName: string, keys: string[]): string => {
    return `:rm ${tableName} {${keys}}`;
  };

  const generateModifyCommand = (
    tableName: string,
    command: 'put' | 'update' = 'put',
    fieldNames: string[] = []
  ): string => {
    const { keys, values } = schema[tableName];
    const hasValues = values.length > 0;

    const actualValues =
      fieldNames.length > 0 ? fieldNames.filter((f) => values.includes(f)) : values;

    return !hasValues
      ? `:${command} ${tableName} {${keys}}`
      : `:${command} ${tableName} {${keys} => ${actualValues}}`;
  };

  const generateAtomCommand = (
    tableName: string,
    items: Partial<DbEntity>[],
    fieldNames: string[] = []
  ): string => {
    const tableSchema = dbSchema[tableName];

    const selectedColumns =
      fieldNames.length > 0
        ? Object.keys(tableSchema.columns).reduce(
            (acc, key) => {
              if (fieldNames.includes(key)) {
                acc[key] = tableSchema.columns[key];
              }
              return acc;
            },
            {} as Record<string, Column>
          )
        : tableSchema.columns;

    const colKeys = Object.keys(selectedColumns);
    const colValues = Object.values(selectedColumns) as Column[];

    return `?[${colKeys.join(', ')}] <- [${items
      .map((item) => entityToArray(item, colValues))
      .join(', ')}]`;
  };

  const generatePut = (tableName: string, array: Partial<DbEntity>[]) => {
    const fields = Object.keys(array[0]);
    const atomCommand = generateAtomCommand(tableName, array, fields);
    const putCommand = generateModifyCommand(tableName, 'put');
    return `${atomCommand}\r\n${putCommand}`;
  };

  const generateRm = (tableName: string, keyValues: Partial<DbEntity>[]) => {
    const { keys } = schema[tableName];

    const atomCommand = generateAtomCommand(tableName, keyValues, keys);
    const rmCommand = generateRmCommand(tableName, keys);
    return `${atomCommand}\r\n${rmCommand}`;
  };

  const generateUpdate = (tableName: string, array: Partial<DbEntity>[]) => {
    // align fields by first entity
    const fields = Object.keys(array[0]);
    const atomCommand = generateAtomCommand(tableName, array, fields);
    const updateCommand = generateModifyCommand(tableName, 'update', fields);
    return `${atomCommand}\r\n${updateCommand}`;
  };

  const generateGet = (
    tableName: string,
    selectFields: string[] = [],
    conditions: string[] = [],
    conditionFields: string[] = [],
    options: GetCommandOptions = {}
  ) => {
    const tableSchema = dbSchema[tableName];

    const queryFields = selectFields.length > 0 ? selectFields : Object.keys(tableSchema.columns);

    const requiredFields = [...queryFields, ...conditionFields];

    const conditionsStr = conditions.length > 0 ? `, ${conditions.join(', ')} ` : '';

    const orderByStr = options.orderBy ? `:order ${options.orderBy}` : '';
    const limitStr = options.limit ? `:limit ${options.limit}` : '';
    const offsetStr = options.limit ? `:offset ${options.offset}` : '';
    return `?[${queryFields.join(', ')}] := *${tableName}{${requiredFields.join(
      ', '
    )}} ${conditionsStr} ${orderByStr} ${limitStr} ${offsetStr}`;
  };

  return {
    generateModifyCommand,
    generateAtomCommand,
    generatePut,
    generateGet,
    generateUpdate,
    generateRm,
  };
}
