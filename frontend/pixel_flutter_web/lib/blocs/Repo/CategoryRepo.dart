import 'package:pixel_flutter_web/blocs/Repo/UserRepo.dart';
import 'package:pixel_flutter_web/env.dart';
import 'package:pixel_flutter_web/models/Category.dart';
import 'package:pixel_flutter_web/api/PixelShareAPI.dart';


class CategoryRepo extends UserRepo with env{
  final _api = PixelShareAPI();

  Future<List<Category>> fetchCategories() async {
    final _categories = await _api.getCategories();
    await saveCategories(categories: _categories);
    return _categories;
  }

  Future<List<Category>> loadCategories() async {
    if (!await hasLocal(key: 'categoryUpdateAt')) {
      return fetchCategories();
    }

    final String _lastUpdateDateString =
        await getLocal(key: 'categoryUpdateAt');

    final int _timeGap =
        DateTime.parse(_lastUpdateDateString).compareTo(DateTime.now());

    if (_timeGap > TIME_GATE) {
      return fetchCategories();
    }

    final List<Category> _categories = [];
    for (var i = 0; i < 999; i++) {
      if (await hasLocal(key: 'category:$i')) {
        final _id = i;
        final _categoryData = await getLocal(key: 'category:$i');
        final _categoryVec = _categoryData.split(':::');
        _categories.add(Category(
            id: _id, name: _categoryVec[0], thumbnail: _categoryVec[1]));
      }
    }
    return _categories;
  }

  Future<void> saveCategories({List<Category> categories}) async {
    final String now = DateTime.now().toString();
    await saveLocal(data: now, key: 'categoryUpdateAt');

    categories.forEach((Category category) async {
      final _id = category.id;
      final _name = category.name;
      final _thumbnail = category.thumbnail;
      final String key = 'category:$_id';
      await saveLocal(data: '$_name:::$_thumbnail', key: key);
    });
  }
}
