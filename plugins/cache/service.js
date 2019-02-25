'use strict'

const { postStringify, postsStringify, topicStringify, userStringify } = require('../../util/fastStringify');

class CacheService {
    constructor(redis) {
        this.redis = redis;
    }

    async refreshPostCache(toTid, postData) {
        
        await this.redis.zremrangebyscore(`toTid:${toTid}`, pid, pid)
        return this.redis.zadd(`toTid:${toTid}`, pid, postData)
    }

    async refreshTopicCache(topicData) {
        const { cid, tid, lastPostTime } = topicData;
        const timeString = new Date(lastPostTime).toISOString();
        const timeScoreNew = Date.parse(timeString);

        const timeScoreOld = await this.redis.zrangebyscore('topics:time', tid, tid);
        if (timeScoreOld.length) {
            await Promise.all([
                await this.redis.zremrangebyscore(`topics:${cid}`, timeScoreOld[0], timeScoreOld[0]),
                await this.redis.zremrangebyscore('topics:time', tid, tid),
            ])
        }
        this.redis.zadd(`topics:${cid}`, timeScoreNew, topicStringify(topicData));
        this.redis.zadd('topics:time', tid, timeScoreNew);
    }
}

module.exports = CacheService;