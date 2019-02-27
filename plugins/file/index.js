'use strict'

module.exports = async function (fastify, opts) {
    fastify.addHook('preHandler', fastify.authPreHandler)
    .addHook('preSerialization', fastify.userPreSerialHandler)
    fastify.post('/upload', uploadHandler)

    fastify.setErrorHandler((error, req, res) => {
        res.send(error);
    })
}

module.exports[Symbol.for('plugin-meta')] = {
    decorators: {
        fastify: [
            'authPreHandler',
            'userPreSerialHandler',
            'fileService'
        ]
    }
}

async function uploadHandler(req, reply) {
    const { uid } = req.user;
    const result = await this.fileService.uploadFile(uid, req);
    return result;
}