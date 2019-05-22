import 'package:flutter_web/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter_web/env.dart';

import 'package:pixel_flutter_web/blocs/UserBlocs.dart';

import 'package:pixel_flutter_web/components/FloatingBarActionIcon.dart';

import 'package:pixel_flutter_web/views/AutenticationPage.dart';

class UserButton extends StatelessWidget with env {
  @override
  Widget build(BuildContext context) {
    return Hero(
        tag: 'auth',
        child: Material(
            //ToDo: in case change background color
            color: Colors.transparent,
            child: bloc(context)));
  }

  Widget bloc(context) {
    return BlocBuilder(
        bloc: BlocProvider.of<UserBloc>(context),
        builder: (BuildContext context, UserState state) {
          if (state is UserLoggedOut) {
            return FloatingBarActionIcon(
                iconSize: 30,
                icon: Icon(Icons.apps),
                onPressed: () => pushToLogin(context: context, state: state));
          }
          if (state is UserLoaded) {
            return FloatingBarActionIcon(
              iconSize: 40,
              icon: CircleAvatar(
                  backgroundImage:
                      NetworkImage(url + '${state.user.avatarUrl}')),
              onPressed: () => showDrawer(context: context),
            );
          }
          return FloatingBarActionIcon(
            iconSize: 30,
            icon: Icon(Icons.apps),
            onPressed: () => pushToRegister(context: context, state: state),
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
