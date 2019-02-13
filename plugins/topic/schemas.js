'use strict'

const topicObject = {
    type: 'object',
    properties: {
        _id: { type: 'string' },
        uid: { type: 'number' },
        cid: { type: 'number' },
        tid: { type: 'number' },
        mainPid: { type: 'number' },
        titleData: { type: 'string' },
    }
}

const addTopic = {
    body: {
        type: 'object',
        required: ['titleData', 'postData'],
        properties: {
            titleData: { type: 'string', minLength: 8, maxLength: 255 },
            postData: { type: 'string', minLength: 8, maxLength: 255 }
        },
        additionalProperties: false
    }
}

module.exports = {
    addTopic,
}