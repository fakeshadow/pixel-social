'use strict'
const fs = require('fs');
const crypto = require('crypto');
const pump = require('pump');

class FileService {
    uploadFile(uid, req) {
        return new Promise((resolve, reject) => {
            const path = [];
            let avatar;
            // set and handle file limits
            const options = {
                limits: {
                    fieldNameSize: 100,
                    fieldSize: 1000000,
                    fields: 10,
                    fileSize: 1000000,
                    files: 5,
                    headerPairs: 2000
                }
            };
            const mp = req.multipart(handler, done, options);
            mp.on('partsLimit', () => reject({
                'error': 'Maximum number of form parts reached'
            }));
            mp.on('filesLimit', () => reject({
                'error': 'Maximum number of files reached'
            }));
            mp.on('fieldsLimit', () => reject({
                'error': 'Maximim number of fields reached'
            }));

            // on success return the array with all file path
            function done(err) {
                if (err) {
                    return reject(err)
                };
                resolve(path.length ? path : avatar);
            }

            function handler(field, file, filename, encoding, mimetype) {
                const array = filename.split('.');
                const index = array.length;
                const extension = array[index - 1];
                if (extension !== 'jpg' && extension !== 'jpeg' && extension !== 'png' && extension !== 'gif') {
                    return reject({
                        'error': 'wrong type'
                    });
                }
                if (field === 'avatar') {
                    pump(file, fs.createWriteStream(`./public/avatar/uid_${uid}.${extension}`));
                    avatar = {
                        "avatar": `/public/avatar/uid_${uid}.${extension}`
                    };
                } else if (field == 'picture') {
                    const date = new Date().getTime();
                    const randomString = crypto.randomBytes(4).toString('hex');
                    pump(file, fs.createWriteStream(`./public/picture/${date}_${randomString}.${extension}`));
                    path.push({
                        "type": "picture",
                        "path": `/public/picture/${date}_${randomString}.${extension}`
                    });
                } else {
                    reject({
                        'error': 'unknown'
                    });
                }
            }
        })
    }
}

module.exports = FileService;