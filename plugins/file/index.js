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
            'fileService'
        ]
    }
}

async function uploadHandler(req, reply) {
    const { uid } = req.user;
    const result = await this.fileService.uploadFile(uid, req);
    return result;
}