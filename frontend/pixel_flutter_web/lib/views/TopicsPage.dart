import 'package:flutter_web/material.dart';

import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter_web/env.dart';

import 'package:pixel_flutter_web/blocs/FloatingButtonBlocs.dart';
import 'package:pixel_flutter_web/blocs/TopicsBlocs.dart';

import 'package:pixel_flutter_web/components/FloatingAppBar.dart';
import 'package:pixel_flutter_web/components/BasicLayout.dart';
import 'package:pixel_flutter_web/components/BasicSliverPadding.dart';
import 'package:pixel_flutter_web/components/TopicsList.dart';
import 'package:pixel_flutter_web/components/SideMenu.dart';

import 'package:pixel_flutter_web/models/Category.dart';

class TopicsPage extends StatefulWidget with env {
  TopicsPage({Key key, this.category}) : super(key: key);

  final Category category;

  @override
  _TopicsPageState createState() => _TopicsPageState();
}

class _TopicsPageState extends State<TopicsPage> {
  TopicsBloc _topicsBloc;
  Category category;
  final _scrollController = ScrollController();
  final _scrollThreshold = 300.0;

  @override
  void initState() {
    BlocProvider.of<FloatingButtonBloc>(context).dispatch(ShowFloating(showFloating: false));
    category = widget.category;
    _topicsBloc = TopicsBloc();
    _topicsBloc.dispatch(GetTopics(categoryId: category.id));
    _scrollController.addListener(_onScroll);
    super.initState();
  }

  @override
  void dispose() {
    _topicsBloc.dispose();
    _scrollController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return BasicLayout(
      scrollView: scrollView(_scrollController),
      sideMenu: SideMenu(),
      backToTop: () => backTop(),
    );
  }

  Widget scrollView(scrollController) {
    return Scrollbar(
      child: CustomScrollView(controller: scrollController, slivers: [
        FloatingAppBar(title: category.name),
        BasicSliverPadding(sliver: TopicsList(topicsBloc: _topicsBloc))
      ]),
    );
  }

  void _onScroll() {
    final maxScroll = _scrollController.position.maxScrollExtent;
    final currentScroll = _scrollController.position.pixels;
    if (maxScroll - currentScroll <= _scrollThreshold) {
      loadMore();
    }
    if (currentScroll > MediaQuery.of(context).size.height) {
      BlocProvider.of<FloatingButtonBloc>(context)
          .dispatch(ShowFloating(showFloating: true));
    }
    if (currentScroll < MediaQuery.of(context).size.height) {
      BlocProvider.of<FloatingButtonBloc>(context)
          .dispatch(ShowFloating(showFloating: false));
    }
  }

  void loadMore() {
    _topicsBloc.dispatch(GetTopics(categoryId: widget.category.id));
  }

  void backTop() {
    _scrollController.animateTo(0.0,
        duration: Duration(milliseconds: 300), curve: Curves.linear);
  }
}
