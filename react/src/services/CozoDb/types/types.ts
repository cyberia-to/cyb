type ColumnType = 'String' | 'Int' | 'Bool' | 'Float' | 'Json';

export interface Column {
  column: string;
  type: ColumnType;
  is_key?: boolean;
  index: number;
  is_default: boolean;
}

export interface TableSchema {
  keys: string[];
  values: string[];
  columns: Record<string, Column>;
}

export type DBValue = string | number | boolean;

export interface IDBResult {
  headers: string[];
  rows: Array<Array<DBValue>>;
  ok: true;
}

interface IDBResultError {
  code: string;
  display: string;
  message: string;
  severity: string;
  ok: false;
}

export class DBResultError extends Error implements Omit<IDBResultError, 'ok'> {
  code: string;

  display: string;

  severity: string;

  constructor(error: IDBResultError) {
    const { message, code, display, severity } = error;
    super(message);
    this.code = code;
    this.display = display;
    this.severity = severity;
  }
}

export type DBSchema = Record<string, TableSchema>;

export type IndexedDbWriteMessage = {
  type: 'indexeddb_write';
  value: number;
};

export type GetCommandOptions = {
  limit?: number;
  offset?: number;
  orderBy?: string[];
};
