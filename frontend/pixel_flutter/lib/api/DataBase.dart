import 'package:path/path.dart';
import 'package:sqflite/sqflite.dart';

import 'package:pixel_flutter/models/Talk.dart';
import 'package:pixel_flutter/models/User.dart';
import 'package:pixel_flutter/models/Category.dart';

const String talkTable = 'CREATE TABLE talks ('
    'id INTEGER PRIMARY KEY,'
    'name TEXT,'
    'description TEXT,'
    'privacy INTEGER,'
    'owner INTEGER,'
    'admin BLOB,'
    'users BLOB);';

const String categoryTable = 'CREATE TABLE categories ('
    'id INTEGER PRIMARY KEY,'
    'name TEXT,'
    'thumbnail TEXT,'
    'postCount INTEGER,'
    'subCount INTEGER,'
    'topicCount INTEGER);';

const String messageTable = 'CREATE TABLE publicMessages ('
    'talk_id INTEGER PRIMARY KEY,'
    'time INTEGER,'
    'message TEXT);';

const String prvMessageTable = 'CREATE TABLE privateMessages ('
    'from_id INTEGER PRIMARY KEY,'
    'time INTEGER,'
    'message TEXT);';

const String usersTable = 'CREATE TABLE users ('
    'id INTEGER PRIMARY KEY,'
    'username TEXT,'
    'email TEXT,'
    'signature TEXT,'
    'avatarUrl TEXT);';

const String keyTable = 'CREATE TABLE keys ('
    'key TEXT PRIMARY KEY,'
    'value TEXT);';

class DataBase {
  static Future<String> pathGen() async {
    final databasePath = await getDatabasesPath();
    return join(databasePath, 'pixeshare.db');
  }

  static Future<Database> getDb() async {
    final path = await pathGen();
    return openDatabase(path);
  }

  static Future<Database> getDbReadOnly() async {
    final path = await pathGen();
    return openReadOnlyDatabase(path);
  }

  static Future<void> delDb() async {
    final path = await pathGen();
    return deleteDatabase(path);
  }

  static Future<void> createDb() async {
    final path = await pathGen();
    final exists = await databaseExists(path);
    if (!exists) {
      print("creating database");
      final Database db = await openDatabase(path, version: 1,
          onCreate: (Database db, int version) async {
        await db.execute(talkTable);
        await db.execute(categoryTable);
        await db.execute(messageTable);
        await db.execute(prvMessageTable);
        await db.execute(keyTable);
        await db.execute(usersTable);
        return;
      });
      return db.close();
    }
  }

  static Future<void> setCategoriesLocal({Database db, List<Category> categories}) async {
    final String now = DateTime.now().toString();

    await setValue(db: db, key: 'categoriesupdate', value: now);

    var batch = db.batch();
//    batch.insert('keys', {'key': 'categoriesupdate', 'value': now},
//        conflictAlgorithm: ConflictAlgorithm.replace);
    for (var cat in categories) {
      batch.insert('categories', cat.toMap(),
          conflictAlgorithm: ConflictAlgorithm.replace);
    }

    await batch.commit(noResult: true);
    return;
  }

  static Future<List<Category>> getCategoriesLocal({Database db}) async {
    final List<Map<String, dynamic>> list = await db.query('categories');
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

  static Future<List<Talk>> getTalksLocal({Database db}) async {
    final List<Map<String, dynamic>> list = await db.query('talks');
    final List<Talk> talks = list.map((col) {
      return Talk(
          id: col['id'],
          name: col['name'],
          description: col['description'],
          privacy: col['privacy'],
          owner: col['owner'],
          admin: col['admin'],
          users: col['users']);
    }).toList();
    return talks;
  }

  static Future<User> getSelfLocal({Database db}) async {
    final username = await getValue(db: db, key: 'username');
    final email = await getValue(db: db, key: 'email');
    final avatarUrl = await getValue(db: db, key: 'avatarUrl');
    final signature = await getValue(db: db, key: 'signature');
    final token = await getValue(db: db, key: 'token');

    return User(
      username: username,
      email: email,
      avatarUrl: avatarUrl,
      signature: signature,
      token: token,
    );
  }

  static Future<void> setSelfLocal({User user, Database db}) async {
    await setValue(db: db, key: 'username', value: user.username);
    await setValue(db: db, key: 'email', value: user.email);
    await setValue(db: db, key: 'avatarUrl', value: user.avatarUrl);
    await setValue(db: db, key: 'signature', value: user.signature);
    await setValue(db: db, key: 'token', value: user.token);
    return;
  }

  static Future<void> saveTalks({List<Talk> talks, Database db}) async {
    final Database db = await getDb();

    var batch = db.batch();
    for (var t in talks) {
      batch.insert('talks', t.toMap(),
          conflictAlgorithm: ConflictAlgorithm.replace);
    }
    return batch.commit(noResult: true);
  }

  static Future<void> delSet({Database db, String key}) async {
    return db.delete('keys', where: '"key" = ?', whereArgs: [key]);
  }

  static Future<void> setValue({Database db, String key, String value}) async {
    await db.insert('keys', {'key': key, 'value': value}, conflictAlgorithm: ConflictAlgorithm.replace);
    return;
  }

  static Future<String> getValue({Database db, String key}) async {
    final List<Map<String, dynamic>> list = await db.query('keys',
        columns: ['value'], where: '"key" = ?', whereArgs: [key]);

    return list.first['value'];
  }
}
