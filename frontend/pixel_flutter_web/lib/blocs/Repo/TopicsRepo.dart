import 'package:pixel_flutter_web/models/Topic.dart';
import 'package:pixel_flutter_web/api/PixelShareAPI.dart';
import 'package:pixel_flutter_web/models/TopicWithPost.dart';

class TopicsRepo {
  final _api = PixelShareAPI();

  Future<List<Topic>> getTopics(int categoryId, int page) async {
    return _api.getTopics(categoryId, page);
  }

  Future<TopicWithPost> getTopic(int topicId, int page) async {
    return _api.getTopic(topicId, page);
  }

  Future<Topic> addTopic(Topic topic, String jwt)  {
    return _api.addTopic(topic, jwt);
  }
}
