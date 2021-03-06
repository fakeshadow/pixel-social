import 'package:flutter_web/widgets.dart';
import 'dart:html';

import 'package:pixel_flutter_web/api/PixelShareAPI.dart';
import 'package:pixel_flutter_web/models/User.dart';

class UserRepo {
  final _api = PixelShareAPI();

  Future<User> register({
    @required String username,
    @required String password,
    @required String email,
  }) async {
    await _api.register(username, password, email);
    return await this.login(username: username, password: password);
  }

  Future<User> login(
      {@required String username, @required String password}) async {
    final User _userData = await _api.login(username, password);
    print(_userData);
    await this.saveUser(_userData);
    return _userData;
  }

  Future<User> update() async {
    await Future.delayed(Duration(seconds: 1));
    return User(id: 1, username: 'test', avatarUrl: 'test', signature: 'test');
  }

  Future<User> getLocalUser() async {
    final _username = await getLocal(key: 'username');
    final _email = await getLocal(key: 'email');
    final _avatar = await getLocal(key: 'avatar');
    final _signature = await getLocal(key: 'signature');
    final _token = await getLocal(key: 'token');

    return User(
        username: _username,
        email: _email,
        avatarUrl: _avatar,
        signature: _signature,
        token: _token);
  }

  Future<void> saveUser(User user) async {
    await saveLocal(data: user.username, key: 'username');
    await saveLocal(data: user.email, key: 'email');
    await saveLocal(data: user.avatarUrl, key: 'avatar');
    await saveLocal(data: user.signature, key: 'signature');
    await saveLocal(data: user.token, key: 'token');
  }

  Future<void> deleteUser() async {
    await deleteLocal(key: 'username');
    await deleteLocal(key: 'email');
    await deleteLocal(key: 'avatar');
    await deleteLocal(key: 'signature');
    await deleteLocal(key: 'token');
  }

  Future<void> deleteToken() async {
    await deleteLocal(key: 'token');
  }

  Future<bool> hasToken() async {
    return hasLocal(key: 'token');
  }

  /// localstorage functions extends for other repo to use
  Future<bool> hasLocal({
    @required String key,
  }) async {
    return window.localStorage.containsKey(key);
  }

  Future<String> getLocal({
    @required String key,
  }) async {
    return window.localStorage[key];
  }

  Future<void> deleteLocal({
    @required String key,
  }) async {
    window.localStorage.remove(key);
  }

  Future<void> saveLocal({
    @required String data,
    @required String key,
  }) async {
    window.localStorage[key] = data;
  }
}
