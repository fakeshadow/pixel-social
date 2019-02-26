'use strict'

async function userPreHook(req, res) {
    try {
        // await this.cacheService.deleteCache();
        const { body } = req;
        const cached = await this.cacheService.getUserCache(body);
        if (cached) res.send(cached);
    } catch (err) {
        res.send(err)
    }
}

async function userPreSerialHook(req, res, payload) {
    try {
        return this.cacheService.refreshUserCache(payload);
    } catch (err) {
        res.send(err)
    }
}

module.exports = {
    userPreHook,
    userPreSerialHook
}
