'use strict';

exports.mapUid = arrayMap => {
    return new Promise(resolve => {
        const uidsMap = [];
        arrayMap.forEach(index => {
            if (uidsMap.indexOf(index.uid) < 0) {
                uidsMap.push(index.uid)
            }
        });
        return resolve(uidsMap);
    })
}

exports.alterPosts = (arrayMap, uidsDetails) => {
    return new Promise(resolve => {
        const result = [];
        arrayMap.forEach(index => {
            uidsDetails.forEach(detail => {
                if (index.uid === detail.uid) {
                    result.push({
                        'pid': index.pid,
                        'toTid': index.toTid,
                        'toPid': index.toPid,
                        'postContent': index.postContent,
                        'postCount': index.postCount,
                        'createdAt': index.createdAt,
                        'user': detail
                    });
                }
            });
        })
        return resolve(result);
    })
}

exports.alterTopics = (arrayMap, uidsDetails) => {
    return new Promise(resolve => {
        const result = [];
        arrayMap.forEach(index => {
            uidsDetails.forEach(detail => {
                if (index.uid === detail.uid) {
                    result.push({
                        'tid': index.tid,
                        'cid': index.cid,
                        'mainPid': index.mainPid,
                        'topicContent': index.topicContent,
                        'lastPostTime': index.lastPostTime,
                        'postCount': index.postCount,
                        'user': detail
                    });
                }
            });
        })
        return resolve(result);
    })
}

exports.parseCache = cache => {
    const array = [];
    return new Promise(resolve => {
        cache.forEach(cache => array.push(JSON.parse(cache)))
        resolve(array);
    })
}

