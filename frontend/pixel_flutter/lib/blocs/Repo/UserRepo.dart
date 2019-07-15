import 'package:flutter/widgets.dart';

import 'package:pixel_flutter/api/DataBase.dart';

import 'package:pixel_flutter/api/PixelShareAPI.dart';
import 'package:pixel_flutter/models/User.dart';
import 'package:sqflite/sqflite.dart';

class UserRepo {

  Future<User> register(
      {@required String username,
      @required String password,
      @required String email,
      @required Database db}) async {
    await PixelShareAPI.register(username, password, email);
    return this.login(username: username, password: password, db: db);
  }

  Future<User> login(
      {@required String username,
      @required String password,
      @required Database db}) async {
    final User _user = await PixelShareAPI.login(username, password);
    await saveUser(db: db, user: _user);
    return _user;
  }

  Future<User> update() async {
    await Future.delayed(Duration(seconds: 1));
    return User(id: 1, username: 'test', avatarUrl: 'test', signature: 'test');
  }

  Future<void> saveUser({Database db, User user}) async {
//    final ddb = await DataBase.getDb();
    return DataBase.setSelf(db: db, user: user);
  }

  Future<void> deleteUser({Database db}) async {
    await DataBase.delSet(db: db, key: 'username');
    await DataBase.delSet(db: db, key: 'email');
    await DataBase.delSet(db: db, key: 'avatarUrl');
    await DataBase.delSet(db: db, key: 'signature');
    await DataBase.delSet(db: db, key: 'token');
    return;
  }

  Future<String> deleteToken({Database db}) async {
    await DataBase.delSet(db: db, key: 'token');
    final username = await DataBase.getValue(db: db, key: 'username');
    return username;
  }

  Future<bool> hasToken({Database db}) async {
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
