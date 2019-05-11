import 'dart:async';
import 'dart:convert';
import 'package:http/http.dart' show Client;
import 'package:pixel_flutter_web/models/Category.dart';
import 'package:pixel_flutter_web/models/Post.dart';
import 'package:pixel_flutter_web/models/Topic.dart';
import 'package:pixel_flutter_web/models/User.dart';
import 'package:pixel_flutter_web/models/TopicWithPost.dart';

class PixelShareAPI {
  final Client _http = Client();
  static const String _url = 'http://192.168.1.197:3200';

  Future<void> register(String username, String password, String email) async {
    try {
      await _http.post('$_url/user/register',
          headers: {"Content-Type": "application/json"},
          body: json.encode(
              {'username': username, 'password': password, 'email': email}));
    } catch (e) {
      throw e;
    }
  }

  Future<User> login(String username, String password) async {
    final response = await _http.post('$_url/user/login',
        headers: {"Content-Type": "application/json"},
        body: json.encode({'username': username, 'password': password}));

    final data = json.decode(response.body);
    return User(
        id: data['user_data']['id'],
        email: data['user_data']['email'],
        username: data['user_data']['username'],
        avatarUrl: data['user_data']['avatar_url'],
        signature: data['user_data']['signature'],
        token: data['token']);
  }

  Future<List<Category>> getCategories() async {
    try {
      final response = await _http.get('$_url/categories/',
          headers: {"Content-Type": "application/json"});
      final data = json.decode(response.body) as List;
      return data.map((rawCategories) {
        return Category(
            id: rawCategories['id'],
            name: rawCategories['name'],
            thumbnail: rawCategories['thumbnail'],
            postCount: rawCategories['post_count'],
            subCount: rawCategories['subscriber_count'],
            topicCount: rawCategories['topic_count']);
      }).toList();
    } catch (e) {
      throw e;
    }
  }

  Future<List<Topic>> getTopics(int categoryId, int page) async {
    try {
      final response = await _http.get('$_url/categories/$categoryId/$page',
          headers: {"Content-Type": "application/json"});
      final data = json.decode(response.body) as List;
      return data.map((rawTopic) {
        return Topic(
            id: rawTopic['id'],
            categoryId: rawTopic['category_id'],
            userId: rawTopic['user']['user_id'],
            username: rawTopic['user']['username'],
            title: rawTopic['title'],
            body: rawTopic['body'],
            lastReplyTime: rawTopic['last_reply_time'],
            avatarUrl: rawTopic['user']['avatar_url'],
            thumbnail: rawTopic['thumbnail']);
      }).toList();
    } catch (e) {
      throw e;
    }
  }

  Future<TopicWithPost> getTopic(int topicId, int page, String token) async {
    try {
      final response = await _http.get('$_url/topic/$topicId/$page', headers: {
        "Content-Type": "application/json",
        "Authorization": "Bearer " + token
      });

      final data = json.decode(response.body);
      final topic = Topic(
          id: data['topic']['id'],
          categoryId: data['topic']['category_id'],
          userId: data['topic']['user']['user_id'],
          username: data['topic']['user']['username'],
          title: data['topic']['title'],
          body: data['topic']['body'],
          lastReplyTime: data['topic']['last_reply_time'],
          avatarUrl: data['topic']['user']['avatar_url'],
          thumbnail: data['topic']['thumbnail']);

      final posts = data['posts'].map((rawPost) {
        return Post(
            id: rawPost['id'],
            userId: rawPost['user']['user_id'],
            username: rawPost['user']['username'],
            avatarUrl: rawPost['user']['avatar_url'],
            topicId: rawPost['topic_id'],
            postId: rawPost['post_id'],
            postContent: rawPost['post_content'],
            lastReplyTime: rawPost['last_reply_time'],
            replyCount: rawPost['reply_count']);
      }).toList();
      return TopicWithPost(topic: topic, posts: posts);
    } catch (e) {
      throw e;
    }
  }
}
