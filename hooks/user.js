'use strict'

async function userPreHandler(req, res) {
    try {
        // await this.cacheService.deleteCache();
        const { body } = req;
        const cached = await this.cacheService.getUserCache(body);
        if (cached) res.send(cached);
    } catch (err) {
        res.send(err)
    }
}

async function userPreSerialHandler(req, res, payload) {
    try {
        // hook for update profile when user upload a new avatar
        const { avatar } = payload;
        const { uid } = req.user;
        if (avatar !== undefined && avatar !== null) {
            const userData = { avatar: avatar };
            const payloadNew = await this.userService.updateProfile(uid, userData);
            return this.cacheService.refreshUserCache(payloadNew);
        }
        return this.cacheService.refreshUserCache(payload);
    } catch (err) {
        res.send(err)
    }
}

module.exports = {
    userPreHandler,
    userPreSerialHandler
}

