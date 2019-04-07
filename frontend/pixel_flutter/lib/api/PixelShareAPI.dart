import 'dart:async';
import 'dart:convert';
import 'package:http/http.dart' show Client;
import 'package:pixel_flutter/models/Post.dart';

class PixelShareAPI {
  final Client _client = Client();
  static const String _url = 'http://192.168.1.197:3100/api/';
  Future<List<Post>> getPosts({
    String topicId = '',
  }) async {
    List<Post> posts = [];

    await _client
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
    await _client
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
