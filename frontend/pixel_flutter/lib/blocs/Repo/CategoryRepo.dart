import 'package:shared_preferences/shared_preferences.dart';

import 'package:pixel_flutter/env.dart';

import 'package:pixel_flutter/models/Category.dart';
import 'package:pixel_flutter/api/PixelShareAPI.dart';

class CategoryRepo with env {
  final _api = PixelShareAPI();

  Future<List<Category>> fetchCategories() async {
    final _categories = await _api.getCategories();
    saveCategories(categories: _categories);
    return _categories;
  }

  Future<List<Category>> loadCategories() async {
    SharedPreferences prefs = await SharedPreferences.getInstance();

    if (!prefs.containsKey('categoryUpdateAt')) {
      return fetchCategories();
    }

    final _lastUpdateDate = prefs.getString('categoryUpdateAt');

    final int _timeGap =
        DateTime.parse(_lastUpdateDate).compareTo(DateTime.now());

    if (_timeGap > TIME_GATE) {
      return fetchCategories();
    }

    final List<Category> _categories = [];
    for (var i = 0; i < 999; i++) {
      if (prefs.containsKey('category:$i')) {
        final _id = i;
        final _categoryData = prefs.getString('category:$i');
        final _categoryVec = _categoryData.split(':::');
        _categories.add(Category(
            id: _id, name: _categoryVec[0], thumbnail: _categoryVec[1]));
      }
    }
    return _categories;
  }

  Future<void> saveCategories({List<Category> categories}) async {
    SharedPreferences prefs = await SharedPreferences.getInstance();
    final String now = DateTime.now().toString();
    await prefs.setString('categoryUpdateAt', now);

    categories.forEach((Category category) async {
      final _id = category.id;
      final _name = category.name;
      final _thumbnail = category.thumbnail;
      final String key = 'category:$_id';
      await prefs.setString(key, '$_name:::$_thumbnail');
    });
  }
}
