'use strict'

const fastJson = require('fast-json-stringify');

const { rawPostObject, postObject } = require('../plugins/post/schemas');
const { rawTopicObject, topicObject } = require('../plugins/topic/schemas');
const { userObject } = require('../plugins/user/schemas');

const postObjects = {
    type: 'array',
    items: postObject
}

const topicObjects = {
    type: 'array',
    items: topicObject
}

const postStringify = fastJson(rawPostObject);
const postsStringify = fastJson(postObjects);
const topicStringify = fastJson(rawTopicObject);
const topicsStringify = fastJson(topicObjects);
const userStringify = fastJson(userObject);

module.exports = {
    postStringify,
    postsStringify,
    topicStringify,
    topicsStringify,
    userStringify,
}