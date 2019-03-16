'use strict'

const fastStringify = require('fast-json-stringify');
const fastParse = require('turbo-json-parse');

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

const postStringify = fastStringify(rawPostObject);
const postsStringify = fastStringify(postObjects);
const topicStringify = fastStringify(rawTopicObject);
const topicsStringify = fastStringify(topicObjects);
const userStringify = fastStringify(userObject);
const postParse = fastParse(rawPostObject, { fullMatch: false, validate: false, });
const topicParse = fastParse(rawTopicObject, { fullMatch: false, validate: false, });
const userParse = fastParse(userObject, { fullMatch: false, validate: false, });


module.exports = {
    postStringify,
    postsStringify,
    topicStringify,
    topicsStringify,
    userStringify,
    postParse,
    topicParse,
    userParse,
}