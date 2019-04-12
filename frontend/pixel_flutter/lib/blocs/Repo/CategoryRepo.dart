import 'package:pixel_flutter/models/Category.dart';
import 'package:pixel_flutter/api/PixelShareAPI.dart';

class CategoryRepo {
  final _api = PixelShareAPI();

  Future<List<Category>> fetchCategories() async {
    return await _api.getCategories();
  }

  Future<void> saveCategories(List<Category> categories) async {
    /// write categories to local storage with a time stamp. use it for further update when new categories are added.
    await Future.delayed(Duration(seconds: 1));
    return;
  }


}
