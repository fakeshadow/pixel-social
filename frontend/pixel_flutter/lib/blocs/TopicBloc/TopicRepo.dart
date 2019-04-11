import 'package:pixel_flutter/models/Topic.dart';
import 'package:pixel_flutter/api/PixelShareAPI.dart';

class TopicRepo {
  final _api = PixelShareAPI();

  Future<List<Topic>> getTopics(int categoryId, int page) async {
    return await _api.getTopics(categoryId, page);
  }
}
