import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:pixel_flutter/blocs/UserBlocs.dart';
import 'package:pixel_flutter/components/Icon/AvatarIcon.dart';

class CatNavBar extends StatelessWidget {
  final String title;

  CatNavBar({
    this.title,
  });

  @override
  Widget build(BuildContext context) {
    return AppBar(
      elevation: 0,
        toolbarOpacity: 1,
        bottomOpacity: 0,
        backgroundColor: Colors.transparent,
        actions: <Widget>[TestIcon(), AvatarIcon()]);
  }
}

class TestIcon extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    final _userBloc = BlocProvider.of<UserBloc>(context);
    return IconButton(
      onPressed: () =>_userBloc.dispatch(Delete()),
      icon: Icon(Icons.time_to_leave),
    );
  }
}
