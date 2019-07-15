import 'dart:core';

import 'package:sqflite/sqlite_api.dart';

import 'package:pixel_flutter/models/Category.dart';

import 'package:pixel_flutter/api/PixelShareAPI.dart';
import 'package:pixel_flutter/api/DataBase.dart';

import 'package:pixel_flutter/env.dart';

CategoryRepo categoryRepo = CategoryRepo();

class CategoryRepo {
  static final CategoryRepo _categoryRepo = CategoryRepo._internal();

  factory CategoryRepo() {
    return _categoryRepo;
  }

  CategoryRepo._internal();

  static Future<List<Category>> loadCategories({Database db}) async {
    final _lastUpdateDate =
        await DataBase.getValue(db: db, key: 'categoriesUpdateAt')
            .catchError((_) {
      return null;
    });

    final int _timeGap = _lastUpdateDate != null
        ? DateTime.now().difference(DateTime.parse(_lastUpdateDate)).inSeconds
        : 0;

    if (_timeGap > env.TIME_GATE || _lastUpdateDate == null) {
      final categories = await PixelShareAPI.getCategories();
      await setCategoriesLocal(db: db, categories: categories);
      print('fetching');
      return categories;
    }

    print('from db');
    final categories = await getCategoriesLocal(db: db);
    return categories;
  }

  static Future<void> setCategoriesLocal(
      {Database db, List<Category> categories}) async {
    final String now = DateTime.now().toString();

    var batch = db.batch();
    batch.insert('keys', {'key': 'categoriesUpdateAt', 'value': now},
        conflictAlgorithm: ConflictAlgorithm.replace);
    for (var cat in categories) {
      batch.insert('categories', cat.toMap(),
          conflictAlgorithm: ConflictAlgorithm.replace);
    }

    return batch.commit(noResult: true);
  }

  static Future<List<Category>> getCategoriesLocal({Database db}) async {
    final List<Map<String, dynamic>> list =
        await db.query('categories', orderBy: 'id ASC');
    final List<Category> categories = list.map((col) {
      return Category(
          id: col['id'],
          name: col['name'],
          thumbnail: col['thumbnail'],
          postCount: col['postCount'],
          subCount: col['subCount'],
          topicCount: col['topicCount']);
    }).toList();

    return categories;
  }
}
