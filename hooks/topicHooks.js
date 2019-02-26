'use strict'

//check if the request content are in cache
async function topicPreHook(req, res) {
    try {
        const { body } = req;
        const cached = await this.cacheService.getTopicsCache(body);
        if (cached) res.send(cached);
    } catch (err) {
        res.send(err)
    }
}

async function topicPreSerialHook(req, res, payload) {
    try {
        const { selfPost, selfTopic } = payload;
        if (Array.isArray(payload)) {
            await this.cacheService.addTopicsCache(payload);
            await this.cacheService.addUsersCache(payload);
            return payload;
        }
        if (selfTopic !== null || undefined) await this.cacheService.refreshTopicCache(selfTopic);
        if (selfPost !== null || undefined) await this.cacheService.refreshPostCache(selfPost);
        return { message: 'success' };
    } catch (err) {
        res.send(err)
    }
}

module.exports = {
    topicPreHook,
    topicPreSerialHook
}
