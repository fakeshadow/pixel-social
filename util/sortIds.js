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

exports.alterArray = (arrayMap, uidsDetails) => {
    return new Promise(resolve => {
        const result = [];
        arrayMap.forEach(index => {
            uidsDetails.forEach(detail => {
                if (index.uid === detail.uid) {
                    index.tid !== undefined ?
                        result.push({
                            'tid': index.tid,
                            'cid': index.cid,
                            'mainPid': index.mainPid,
                            'topicContent': index.topicContent,
                            'lastPostTime': index.lastPostTime,
                            'postCount': index.postCount,
                            'user': detail
                        }) : result.push({
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


