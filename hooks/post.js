'use strict'

async function postPreHandler(req, res) {
    try {
        const { body } = req;
        const cached = await this.cacheService.getPostsCache(body);
        if (cached) res.send(cached);
    } catch (err) {
        res.send(err)
    }
}

async function postPreSerialHandler(req, res, payload) {
    try {
        const { selfPost, relatedPost, relatedTopic } = payload;
        if (Array.isArray(payload)) {
            await this.cacheService.addPostsCache(payload);
            await this.cacheService.addUsersCache(payload);
            return payload;
        }

        if (selfPost !== null || undefined) await this.cacheService.refreshPostCache(selfPost);
        if (relatedPost !== null || undefined) await this.cacheService.refreshPostCache(relatedPost);
        if (relatedTopic !== null || undefined) await this.cacheService.refreshTopicCache(relatedTopic);
        return { message: 'success' };
    } catch (err) {
        res.send(err)
    }
}

module.exports = {
    postPreHandler,
    postPreSerialHandler
}
