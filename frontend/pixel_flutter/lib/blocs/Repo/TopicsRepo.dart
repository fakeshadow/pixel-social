import 'package:pixel_flutter/models/Topic.dart';
import 'package:pixel_flutter/models/TopicWithPost.dart';
import 'package:pixel_flutter/api/PixelShareAPI.dart';

class TopicsRepo {

  static Future<List<Topic>> getTopics(int categoryId, int page) async {
    return PixelShareAPI.getTopics(categoryId, page);
  }

  static Future<TopicWithPost> getTopic(int topicId, int page) async {
    return PixelShareAPI.getTopic(topicId, page);
  }

  static Future<Topic> addTopic(Topic topic, String jwt) {
    return PixelShareAPI.addTopic(topic, jwt);
  }
}
