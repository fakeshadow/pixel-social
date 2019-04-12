import 'package:flutter/widgets.dart';
import 'package:pixel_flutter/api/PixelShareAPI.dart';
import 'package:pixel_flutter/models/User.dart';

class UserRepo {
  final _api = PixelShareAPI();

  Future<User> register({@required String username, @required String password, @required String email}) async {
    await _api.register(username, password, email);
    return await _api.login(username, password);
  }

  Future<User> login({@required String username, @required String password}) async {
    return await _api.login(username, password);
  }

  Future<User> update() async {
    await Future.delayed(Duration(seconds: 1));
    return User(id: 1, username: 'test', avatarUrl: 'test', signature: 'test');
  }

  Future<void> logout() async {
    /// delete user info and token
    await Future.delayed(Duration(seconds: 1));
    return;
  }

  Future<User> getLocalUser() async {
    /// save user info and token
    await Future.delayed(Duration(seconds: 1));
    return User(id: 1, username: 'test', avatarUrl: 'test', signature: 'test');
  }

  Future<void> saveUser(User user) async {
    /// save user locally
    await Future.delayed(Duration(seconds: 1));
    return;
  }

  Future<void> deleteToken() async {
    /// delete token locally
    await Future.delayed(Duration(seconds: 1));
    return;
  }

  Future<bool> hasToken() async {
    /// check if there is local token
    await Future.delayed(Duration(seconds: 1));
    return true;
  }
}
