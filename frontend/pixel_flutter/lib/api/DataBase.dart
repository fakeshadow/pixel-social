import 'package:path/path.dart';

import 'package:sqflite/sqflite.dart';

import 'package:pixel_flutter/models/Talk.dart';
import 'package:pixel_flutter/models/User.dart';
import 'package:pixel_flutter/models/Message.dart';

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
    'talk_id INTEGER,'
    'user_id INTEGER PRIMARY KEY,'
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
        await db.execute(keyTable);
        await db.execute(usersTable);
        return;
      });
      return db.close();
    }
  }

  static Future<List<Message>> getMsg(
      {Database db, int talkId, int userId, int time}) async {
    final query = talkId != null
        ? 'SELECT * FROM messages WHERE time <= $time AND talkId = $talkId ORDER BY time DESC limit 20'
        : 'SELECT * FROM messages WHERE time <= $time AND userId = $userId ORDER BY time DESC limit 20';

    final List<Map<String, dynamic>> list = await db.rawQuery(query);

    return list.map((col) {
      final int timeStamp = col['time'];
      return Message(
          talkId: col['talk_id'],
          userId: col['user_id'],
          dateTime: DateTime.fromMillisecondsSinceEpoch(timeStamp),
          msg: col['message']);
    }).toList();
  }

  static Future<List<Talk>> getTalks({Database db}) async {
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

  static Future<User> getSelfUser({Database db}) async {
    final username = await getValue(db: db, key: 'username');
    final email = await getValue(db: db, key: 'email');
    final avatarUrl = await getValue(db: db, key: 'avatarUrl');
    final signature = await getValue(db: db, key: 'signature');
    final token = await getValue(db: db, key: 'token').catchError((_) {
      return null;
    });

    return User(
      username: username,
      email: email,
      avatarUrl: avatarUrl,
      signature: signature,
      token: token,
    );
  }

  static Future<void> setSelfUser({User user, Database db}) async {
    setKeyValue(db: db, key: 'username', value: user.username);
    setKeyValue(db: db, key: 'email', value: user.email);
    setKeyValue(db: db, key: 'avatarUrl', value: user.avatarUrl);
    setKeyValue(db: db, key: 'signature', value: user.signature);
    setKeyValue(db: db, key: 'token', value: user.token);
    return;
  }

  static Future<void> setTalks({List<Talk> talks, Database db}) async {
    var batch = db.batch();
    for (var t in talks) {
      batch.insert('talks', t.toMap(),
          conflictAlgorithm: ConflictAlgorithm.replace);
    }
    return batch.commit(noResult: true);
  }

  static Future<void> setMsg({List<Message> msg, Database db}) async {
    var batch = db.batch();
    for (var m in msg) {
      batch.insert('messages', m.toMap(),
          conflictAlgorithm: ConflictAlgorithm.replace);
    }
    return batch.commit(noResult: true);
  }

  static Future<void> delKeyValue({Database db, String key}) async {
    return db.delete('keys', where: '"key" = ?', whereArgs: [key]);
  }

  static Future<void> setKeyValue(
      {Database db, String key, String value}) async {
    return db.insert('keys', {'key': key, 'value': value},
        conflictAlgorithm: ConflictAlgorithm.replace);
  }

  static Future<String> getValue({Database db, String key}) async {
    final List<Map<String, dynamic>> list = await db.query('keys',
        columns: ['value'], where: '"key" = ?', whereArgs: [key]);

    return list.first['value'];
  }
}
