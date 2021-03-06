import 'package:flutter/material.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:pixel_flutter/Views/AutenticationPage.dart';

import 'package:pixel_flutter/blocs/UserBlocs.dart';
import 'package:pixel_flutter/components/Icon/AvatarIcon.dart';

/// authentication page logic here
class UserButton extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return Hero(tag: 'auth', child: Material(child: bloc(context)));
  }

  Widget bloc(context) {
    return BlocBuilder(
        bloc: BlocProvider.of<UserBloc>(context),
        builder: (BuildContext context, UserState state) {
          if (state is UserLoggedOut) {
            return AvatarIcon(
                showAvatar: false,
                avatarUrl: '',
                callback: () => pushToLogin(context: context, state: state));
          }
          if (state is UserLoaded) {
            return AvatarIcon(
              showAvatar: true,
              avatarUrl: state.user.avatarUrl,
              callback: () => showDrawer(context: context),
            );
          }
          return AvatarIcon(
            showAvatar: false,
            avatarUrl: '',
            callback: () => pushToRegister(context: context, state: state),
          );
        });
  }

  void showDrawer({BuildContext context}) {
    Scaffold.of(context).openEndDrawer();
  }

  void pushToLogin({BuildContext context, state}) {
    Navigator.push(
        context,
        MaterialPageRoute(
            builder: (context) => AuthenticationPage(
                  type: 'Login',
                  username: state.username,
                )));
  }

  void pushToRegister({BuildContext context, state}) {
    Navigator.push(
        context,
        MaterialPageRoute(
            builder: (context) => AuthenticationPage(
                  type: 'Register',
                )));
  }
}
