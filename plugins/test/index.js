'use strict'

module.exports = async (fastify, opts) => {
    fastify.addHook('preHandler', fastify.cachePreHandler);
    fastify.get('/', testHandler)

    fastify.setErrorHandler((error, req, res) => {
        res.send(error);
    })
}


async function testHandler() {
    const cache = '1';
    return this.cacheService.isCached(cache);
}