import 'package:pixel_flutter_web/models/Category.dart';
import 'package:pixel_flutter_web/api/PixelShareAPI.dart';

const TIME_GATE = 360000;

class CategoryRepo {
  final _api = PixelShareAPI();

  Future<List<Category>> fetchCategories() async {
//    try {
//      final _categories = await _api.getCategories();
//      saveCategories(categories: _categories);
//      return _categories;
//    } catch (e) {
//      throw e;
//    }
    return [];
  }

  Future<List<Category>> loadCategories() async {
//    try {
//      SharedPreferences prefs = await SharedPreferences.getInstance();
//      if (!prefs.containsKey('categoryUpdateAt')) {
//        return fetchCategories();
//      }
//
//      final int now = DateTime.now().millisecondsSinceEpoch;
//      final _lastUpdateDate = prefs.getInt('categoryUpdateAt');
//      if (now - _lastUpdateDate > TIME_GATE) {
//        return fetchCategories();
//      }
//
//      final List<Category> _categories = [];
//      for (var i = 0; i < 999; i++) {
//        if (prefs.containsKey('category:$i')) {
//          final _id = i;
//          final _categoryData = prefs.getString('category:$i');
//          final _categoryVec = _categoryData.split(':::');
//          _categories.add(
//              Category(id: _id, name: _categoryVec[0], thumbnail: _categoryVec[1]));
//        }
//      }
//      return _categories;
//    } catch (e) {
//      throw e;
//    }
    return [];
  }

  Future<void> saveCategories({List<Category> categories}) async {
//    try {
//      SharedPreferences prefs = await SharedPreferences.getInstance();
//      final int now = DateTime.now().millisecondsSinceEpoch;
//      await prefs.setInt('categoryUpdateAt', now);
//
//      categories.forEach((Category category) async {
//        final _id = category.id;
//        final _name = category.name;
//        final _thumbnail = category.thumbnail;
//        final String key = 'category:$_id';
//        await prefs.setString(key, '$_name:::$_thumbnail');
//      });
//    } catch (e) {
//      throw e;
//    }
  }
}
