import 'package:pixel_flutter_web/models/Topic.dart';
import 'package:pixel_flutter_web/api/PixelShareAPI.dart';

class TopicRepo {
  final _api = PixelShareAPI();

  Future<List<Topic>> getTopics(int categoryId, int page) async {
    return await _api.getTopics(categoryId, page);
  }
}
