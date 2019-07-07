import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:pixel_flutter/blocs/ErrorBlocs.dart';
import 'package:pixel_flutter/components/Button/UserButton.dart';
import 'package:pixel_flutter/env.dart';
import 'package:pixel_flutter/style/colors.dart';
import 'package:pixel_flutter/components/Icon/SearchIcon.dart';

// ToDo: SliverNavBar rebuild multiple times with unknown reason
class SliverNavBar extends StatelessWidget with env{
  final String title;
  final String thumbnail;

  SliverNavBar({
    this.title,
    this.thumbnail
  });

  @override
  Widget build(BuildContext context) {
    return SliverAppBar(
        leading: IconButton(
          color: primaryColor,
          icon: Icon(Icons.arrow_back),
          tooltip: 'Go back',
          onPressed: (){
            BlocProvider.of<ErrorBloc>(context).dispatch(HideSnack());
            Navigator.of(context).pop();
          },
        ),
        floating: true,
        elevation: 0,
        snap: true,
        backgroundColor: Colors.transparent,
        title: FadeInImage.assetNetwork(
            placeholder: 'assets/category_default_cover.png',
            image: url + "public/" + thumbnail,
            fit: BoxFit.cover,
          ),
        centerTitle: true,
        actions: <Widget>[SearchIcon(), UserButton()]);
  }
}
