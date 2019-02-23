'use strict';

const crypto = require('crypto');

exports.saltHashPassword = password => {
    const salt = genRandomString(32);
    return sha256(password, salt);
}

exports.checksaltHashPassword = async (saltedpassword, password) => {
    const array = saltedpassword.split(':');
    return saltedpassword === await sha256(password, array[0]);
}

const genRandomString = length => {
    return crypto.randomBytes(Math.ceil(length / 2)).toString('hex').slice(0, length);
};

const sha256 = (password, salt) => {
    return new Promise((resolve => {
        crypto.pbkdf2(password, salt, 100, 64, 'sha256', (err, crypted) => {
            if (err) throw err;
            const salted = crypted.toString('hex')
            resolve(salt + ':' + salted);
        })
    }))
};