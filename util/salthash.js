'use strict';
const crypto = require('crypto');

exports.saltHashPassword = password => {
    const salt = genRandomString(32);
    return sha256(password, salt);
}

exports.checksaltHashPassword = (saltedpassword, password) => {
    const array = saltedpassword.split(':');
    return saltedpassword === sha256(password, array[0]);
}

const genRandomString = length => {
    return crypto.randomBytes(Math.ceil(length / 2))
        .toString('hex')
        .slice(0, length);
};

const sha256 = (password, salt) => {
    const hash = crypto.createHmac('sha256', salt);
    hash.update(password);
    const value = hash.digest('hex');
    return salt + ':' + value;
};


