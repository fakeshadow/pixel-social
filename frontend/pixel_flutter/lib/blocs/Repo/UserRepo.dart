import 'package:flutter/widgets.dart';

import 'package:pixel_flutter/api/DataBase.dart';

import 'package:pixel_flutter/api/PixelShareAPI.dart';
import 'package:pixel_flutter/models/User.dart';
import 'package:sqflite/sqflite.dart';

import 'package:pixel_flutter/env.dart';

class UserRepo {
  static Future<User> register(
      {@required String username,
      @required String password,
      @required String email,
      @required Database db}) async {
    await PixelShareAPI.register(username, password, email);
    return login(username: username, password: password, db: db);
  }

  static Future<User> login(
      {@required String username,
      @required String password,
      @required Database db}) async {
    final User _user = await PixelShareAPI.login(username, password);
    setSelf(db: db, user: _user).catchError((_) => env.STORAGE_FAIL);
    return _user;
  }

  static Future<User> update() async {
    await Future.delayed(Duration(seconds: 1));
    return User(id: 1, username: 'test', avatarUrl: 'test', signature: 'test');
  }

  static Future<void> setSelf({Database db, User user}) async {
    return DataBase.setSelfUser(db: db, user: user)
        .catchError((_) => env.STORAGE_FAIL);
  }

  static Future<User> getSelf({Database db}) async {
    return DataBase.getSelfUser(db: db).catchError((_) => env.STORAGE_FAIL);
  }

  static Future<void> deleteUser({Database db}) async {
    try {
      DataBase.delKeyValue(db: db, key: 'username');
      DataBase.delKeyValue(db: db, key: 'email');
      DataBase.delKeyValue(db: db, key: 'avatarUrl');
      DataBase.delKeyValue(db: db, key: 'signature');
      DataBase.delKeyValue(db: db, key: 'token');
    } catch (_) {
      throw (env.STORAGE_FAIL);
    }
  }

  static Future<String> deleteToken({Database db}) async {
    try {
      DataBase.delKeyValue(db: db, key: 'token');
      final username = await DataBase.getValue(db: db, key: 'username');
      return username;
    } catch (_) {
      throw (env.STORAGE_FAIL);
    }
  }

  static Future<bool> hasToken({Database db}) async {
    return DataBase.getValue(db: db, key: 'token').then((token) {
      if (token != null) {
        return true;
      } else {
        return false;
      }
    }).catchError((e) {
      return false;
    });
  }
}
