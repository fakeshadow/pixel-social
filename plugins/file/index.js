'use strict'

const {
    upload: uploadSchema,
} = require('./schemas')

module.exports = async function (fastify, opts) {
    fastify.addHook('preHandler', fastify.authPreHandler)
    fastify.post('/upload', uploadHandler)

    fastify.setErrorHandler((error, req, res) => {
        res.send(error);
    })
}

module.exports[Symbol.for('plugin-meta')] = {
    decorators: {
        fastify: [
            'authPreHandler',
            'fileService',
            'userService'
        ]
    }
}

async function uploadHandler(req, reply) {
    const { uid } = req.user;
    const result = await this.fileService.uploadFile(uid, req);
    if (result[0].type === 'avatar') {
        return await this.userService.updateProfile(uid, result[0]);
    }
    if (result[0].type === 'picture') {
        return result;
    }
}

