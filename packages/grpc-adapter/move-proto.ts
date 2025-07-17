import { cp } from 'fs';
import { join } from 'path';

const copyProtoFiles = () =>
  new Promise((resolve, reject) => {
    const protoFiles = cp(
      join(__dirname, '../../server/proto/workflow_service.proto'),
      join(__dirname, 'proto/workflow_service.proto'),
      (err) => {
        if (err) {
          console.error(err);
          reject(err);
        }
        resolve(protoFiles);
      }
    );
  });

(async () => {
  await copyProtoFiles();
  console.log('Proto files copied');
})();
