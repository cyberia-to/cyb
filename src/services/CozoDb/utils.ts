/* eslint-disable @typescript-eslint/no-explicit-any */
/* eslint-disable import/no-unused-modules */
// eslint-disable-next-line @typescript-eslint/no-unused-vars
import { entityToDto } from 'src/utils/dto';
import { serializeString } from 'src/utils/string';
import { DbEntity } from './types/entities';
import { Column, IDBResult } from './types/types';

export function dbResultToDtoList<T>(dbResult: IDBResult): T[] {
  const { headers, rows } = dbResult;

  // const camelCaseHeadersMap = headers.reduce(
  //   (acc: Record<string, any>, header) => {
  //     acc[header] = snakeToCamel(header);
  //     return acc;
  //   },
  //   {}
  // );

  return rows.map((row) => {
    const obj: Record<string, any> = {};
    headers.forEach((header, index) => {
      // obj[camelCaseHeadersMap[header]] = row[index];
      obj[header] = row[index];
    });

    return entityToDto(obj) as T;
  });
}

export async function clearIndexedDBStore(dbName: string, storeName: string): Promise<void> {
  return new Promise((resolve, reject) => {
    // Open a connection to the database
    const request = indexedDB.open(dbName);

    request.onerror = (_event) => {
      reject(new Error(`Database error: ${request.error?.message}`));
    };

    request.onsuccess = (_event) => {
      const db = request.result;

      // Start a transaction and get the store
      const transaction = db.transaction(storeName, 'readwrite');
      const store = transaction.objectStore(storeName);

      // Clear the store
      const clearRequest = store.clear();

      clearRequest.onsuccess = () => {
        console.log(`Store cleared ${dbName}/${storeName}`);
        resolve();
      };

      clearRequest.onerror = () => {
        reject(new Error(`Error clearing the store: ${clearRequest.error?.message}`));
      };

      // Close the database when the transaction is complete
      transaction.oncomplete = () => {
        db.close();
      };
    };
  });
}
export const toListOfObjects = <T extends Record<string, any>>({
  rows,
  headers,
}: IDBResult): T[] => {
  return rows.map((row) => {
    const obj: Record<string, any> = {};
    row.forEach((value, index) => {
      const key = headers[index];
      obj[key] = value;
    });
    return obj as T;
  });
};
export const entityToArray = (obj: Partial<DbEntity>, columns: Column[]): string => {
  return `[${columns
    .map((col) => {
      const key = col.column as keyof DbEntity;
      const value = obj[key];

      if (/<[a-z]+\d*;\d+>/i.test(col.type)) {
        return `[${value}]`;
      }
      return col.type === 'Json'
        ? `parse_json('${JSON.stringify(value)}')`
        : col.type === 'String'
          ? `"${serializeString(value)}"`
          : value;
    })
    .join(', ')}]`;
};
