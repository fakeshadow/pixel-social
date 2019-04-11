import 'dart:async';
import 'dart:convert';
import 'package:http/http.dart' show Client;
import 'package:pixel_flutter/models/Category.dart';
import 'package:pixel_flutter/models/Post.dart';
import 'package:pixel_flutter/models/Topic.dart';

class PixelShareAPI {
  final Client _http = Client();
  static const String _url = 'http://192.168.1.197:3200';

  Future<List<Category>> getCategories() async {
    final response = await _http.get('$_url/categories/',
        headers: {"Content-Type": "application/json"});

    if (response.statusCode == 200) {
      final data = json.decode(response.body) as List;
      return data.map((rawCategories) {
        return Category(
            id: rawCategories['id'],
            name: rawCategories['name'],
            theme: rawCategories['theme']);
      }).toList();
    } else {
      throw Exception('error getting Topics');
    }
  }

  Future<List<Topic>> getTopics(int categoryId, int page) async {
    final response = await _http
        .get('$_url/categories/$categoryId/$page', headers: {
      "Content-Type": "application/json"
    });

    if (response.statusCode == 200) {
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
    } else {
      throw Exception('error getting Topics');
    }
  }

  Future<List<Post>> getPosts({
    String topicId = '',
  }) async {
    List<Post> posts = [];

    await _http
        .get(Uri.parse(_url + "post/test"))
        .then((res) => res.body)
        .then(json.decode)
        .then((json) => json.forEach((post) {
              Post po = Post.fromJson(post);
              posts.add(po);
            }))
        .catchError((err) => print(err));
    return posts;
  }

  Future<String> postPost({
    String topicId,
    String titleData,
    String postData,
  }) async {
    await _http
        .post(Uri.parse(_url + "topic"), headers: {
          "Content-Type": "application/json"
        }, body: {
          "cid": topicId,
          "titleData": titleData,
          "postData": postData
        })
        .then((res) => print(res))
        .catchError((err) => print(err));
    return 'Success';
  }
}
