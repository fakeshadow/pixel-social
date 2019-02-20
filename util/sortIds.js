'use strict';

exports.mapUid = arrayMap => {
    return new Promise((resolve) => {
        const uidsMap = [];
        arrayMap.forEach(index => {
            if (uidsMap.indexOf(index.uid) < 0) {
                uidsMap.push(index.uid)
            }
        });
        resolve(uidsMap);
    });
}

exports.alterArray = (arrayMap, uidsDetails) => {
    return new Promise((resolve) => {
        arrayMap.map(index => {
            uidsDetails.forEach(detail => {
                if (index.uid === detail.uid) {
                    index["uid"] = index["user"];
                    index.user = detail;
                }
            });
            return index;
        });
        return resolve(arrayMap);
    });
}