import 'dart:core';

import 'package:sqflite/sqlite_api.dart';

import 'package:pixel_flutter/models/Category.dart';

import 'package:pixel_flutter/api/PixelShareAPI.dart';
import 'package:pixel_flutter/api/DataBase.dart';

import 'package:pixel_flutter/env.dart';

CategoryRepo categoryRepo = CategoryRepo();

class CategoryRepo {
  static final _api = PixelShareAPI();

  static final CategoryRepo _categoryRepo = CategoryRepo._internal();

  factory CategoryRepo() {
    return _categoryRepo;
  }

  CategoryRepo._internal();

  static Future<List<Category>> loadCategories({Database db}) async {
    final _lastUpdateDate =
        await DataBase.getValue(db: db, key: 'categoriesupdate')
            .catchError((_) {
      return null;
    });

    final int _timeGap = _lastUpdateDate != null
        ? DateTime.now().difference(DateTime.parse(_lastUpdateDate)).inSeconds
        : 0;

    if (_timeGap > env.TIME_GATE || _lastUpdateDate == null) {
      final categories = await _api.getCategories();
      await DataBase.setCategoriesLocal(db: db, categories: categories);
      print('fetching');
      return categories;
    }

    print('from db');
    final categories = await DataBase.getCategoriesLocal(db: db);
    return categories;
  }
}
