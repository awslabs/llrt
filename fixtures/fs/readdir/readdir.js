import fs from 'fs/promises';

fs.readdir('./', { recursive: true }).then(res => {
    console.log(res);
});
