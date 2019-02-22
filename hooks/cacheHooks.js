'use strict'

// check if the request posts are in cache
async function cachePreHook(req, res) {
    try {
        // await this.redis.flushall();
        const { type } = req.body;
        const { uid } = req.user;
        if (type === 'getPosts') {
            const { toTid, page } = req.body;
            const cache = await this.redis.hgetall(`toTid:${toTid}`);
            console.log(cache)
            res.send(cache);
        }
    } catch (err) {
        res.send(err)
    }
}


async function cachePreSerialHook(req, res, payload) {
    try {
        const { type } = req.body;
        if (type === 'getPosts' || type === 'addPost' || type === 'editPost') {
            const { cache, database } = payload;
            cache.forEach(post => {
                const { pid, toTid } = post;
                const po = JSON.stringify(post);
                this.redis
                    .hmset(`toTid:${toTid}`, `pid:${pid}`, po)
                    .catch(err => console.log(err));
            })
            return { 'cache': [], 'database': database };
        }
    } catch (err) {
        res.send(err)
    }
}

module.exports = {
    cachePreHook,
    cachePreSerialHook
}

